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

import { homedir } from "node:os";
import { basename, dirname, join, relative, sep } from "node:path";

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

/**
 * Resolved, normalized deny entries. Each is either an absolute path prefix
 * (directory or exact file) matched with trailing-separator semantics, or an
 * exact filename matched anywhere under the home directory.
 */
interface DenyRule {
	/** Absolute prefix to match (path must be equal to or inside this). */
	prefix?: string;
	/** Absolute exact path to match. */
	exact?: string;
}

const HOME = homedir();

/** Directories whose entire subtree is denied for writes (persistence/credentials). */
const DENIED_WRITE_DIRS = [
	join(HOME, ".ssh"),
	join(HOME, ".aws"),
	join(HOME, ".gnupg"),
	join(HOME, ".config", "hamr"),
	join(HOME, ".hamr"),
	"/etc",
	// cron/at persistence spools (NOT all of /var — macOS uses /var/folders for temp)
	"/var/spool/cron",
	"/var/spool/at",
	"/var/at",
	"/var/cron",
	"/usr/local/etc",
	join(HOME, ".config", "systemd"),
	join(HOME, ".config", "launchd"),
	join(HOME, "Library", "LaunchAgents"),
	join(HOME, "Library", "LaunchDaemons"),
];

/** Exact files denied for writes (shell rc / persistence / git hooks entrypoints). */
const DENIED_WRITE_FILES = [
	join(HOME, ".bashrc"),
	join(HOME, ".bash_profile"),
	join(HOME, ".profile"),
	join(HOME, ".zshrc"),
	join(HOME, ".zprofile"),
	join(HOME, ".zshenv"),
	join(HOME, ".zlogin"),
	join(HOME, ".config", "fish", "config.fish"),
	join(HOME, ".npmrc"),
	join(HOME, ".netrc"),
	join(HOME, ".gitconfig"),
];

/**
 * Filenames that are denied for writes anywhere on the path (these are
 * persistence entrypoints that may live in project-local dirs too).
 */
const DENIED_WRITE_BASENAMES = new Set(["authorized_keys", "authorized_keys2"]);

/** Exact credential files denied for reads. */
const DENIED_READ_FILES = [
	join(HOME, ".ssh", "id_rsa"),
	join(HOME, ".ssh", "id_ecdsa"),
	join(HOME, ".ssh", "id_ed25519"),
	join(HOME, ".ssh", "id_dsa"),
	join(HOME, ".ssh", "id_ecdsa_sk"),
	join(HOME, ".ssh", "id_ed25519_sk"),
	join(HOME, ".aws", "credentials"),
	join(HOME, ".aws", "config"),
	join(HOME, ".config", "hamr", "auth.json"),
	join(HOME, ".hamr", "auth.json"),
	join(HOME, ".netrc"),
	join(HOME, ".bash_history"),
	join(HOME, ".zsh_history"),
	join(HOME, ".zhistory"),
	join(HOME, ".mysql_history"),
	join(HOME, ".psql_history"),
	join(HOME, ".python_history"),
	join(HOME, ".node_repl_history"),
	join(HOME, ".npm", "_cacache"),
	join(HOME, ".docker", "config.json"),
	join(HOME, ".kube", "config"),
];

/** Private-key basenames denied for reads anywhere under ~/.ssh. */
const DENIED_READ_KEY_BASENAMES = new Set([
	"id_rsa",
	"id_ecdsa",
	"id_ed25519",
	"id_dsa",
	"id_ecdsa_sk",
	"id_ed25519_sk",
]);

function isInside(candidate: string, prefix: string): boolean {
	if (candidate === prefix) return true;
	const rel = relative(prefix, candidate);
	return rel !== "" && !rel.startsWith(`..${sep}`) && !rel.startsWith("..") && !candidate.startsWith(`..${sep}`);
}

function buildDenyRules(): { dirs: string[]; files: string[] } {
	return {
		dirs: DENIED_WRITE_DIRS,
		files: DENIED_WRITE_FILES,
	};
}

const WRITE_RULES = buildDenyRules();

/**
 * Normalize a path for comparison. Does NOT follow symlinks (callers already
 * resolve via resolveToCwd; symlink-chasing would be expensive and is handled
 * by the OS at access time). Returns the path unchanged if it is already
 * absolute.
 */
function normalizeForCompare(p: string): string {
	return p;
}

export class PathGuard {
	readonly enabled: boolean;
	private readonly allowed: string[];
	private readonly strictCwd: string | undefined;

	constructor(options?: PathGuardOptions) {
		this.enabled = options?.enabled ?? true;
		this.allowed = (options?.allowedPaths ?? []).map(normalizeForCompare);
		this.strictCwd = options?.strictCwd ? normalizeForCompare(options.strictCwd) : undefined;
	}

	/**
	 * Returns true if the candidate path is NOT within the strict cwd (if set).
	 * When strictCwd is undefined, this always returns false (no restriction).
	 */
	private isOutsideStrictCwd(candidate: string): boolean {
		if (!this.strictCwd) return false;
		return !isInside(candidate, this.strictCwd);
	}

	private isExplicitlyAllowed(candidate: string): boolean {
		for (const a of this.allowed) {
			if (candidate === a || isInside(candidate, a)) return true;
		}
		return false;
	}

	/**
	 * Returns a denial reason if the write/edit target is forbidden, or
	 * `undefined` if it is allowed.
	 */
	deniedWriteReason(absolutePath: string): string | undefined {
		if (!this.enabled) return undefined;
		const candidate = normalizeForCompare(absolutePath);
		if (this.isExplicitlyAllowed(candidate)) return undefined;

		// .git/hooks/** — git-hook persistence vector.
		if (isGitHooksDir(candidate)) {
			return `Refusing to write inside a git hooks directory (${dirname(candidate)}). This is a common persistence vector.`;
		}

		for (const dir of WRITE_RULES.dirs) {
			if (isInside(candidate, dir)) {
				return `Refusing to write inside sensitive directory (${dir}). This path is on the path-guard denylist.`;
			}
		}
		for (const file of WRITE_RULES.files) {
			if (candidate === file) {
				return `Refusing to overwrite sensitive file (${file}). This path is on the path-guard denylist.`;
			}
		}
		const name = basename(candidate);
		if (DENIED_WRITE_BASENAMES.has(name) && isInside(candidate, join(HOME, ".ssh"))) {
			return `Refusing to write '${name}' inside ~/.ssh.`;
		}
		return undefined;
	}

	/**
	 * Returns a denial reason if the read target is a credential file, or
	 * `undefined` if it is allowed.
	 */
	deniedReadReason(absolutePath: string): string | undefined {
		if (!this.enabled) return undefined;
		const candidate = normalizeForCompare(absolutePath);
		if (this.isExplicitlyAllowed(candidate)) return undefined;

		for (const file of DENIED_READ_FILES) {
			if (candidate === file) {
				return `Refusing to read credential file (${file}). This path is on the path-guard denylist.`;
			}
		}
		// SSH private keys anywhere under ~/.ssh.
		const sshDir = join(HOME, ".ssh");
		if (isInside(candidate, sshDir)) {
			const name = basename(candidate);
			if (DENIED_READ_KEY_BASENAMES.has(name) || name.startsWith("id_")) {
				return `Refusing to read SSH private key (${candidate}).`;
			}
		}
		// ~/.gnupg/** holds private key material.
		if (isInside(candidate, join(HOME, ".gnupg"))) {
			return `Refusing to read inside ~/.gnupg (private key material).`;
		}
		return undefined;
	}

	/** Assert a write target is allowed, throwing with a clear message if not. */
	assertWritable(absolutePath: string): void {
		if (this.isOutsideStrictCwd(absolutePath)) {
			throw new PathGuardError(
				`Path '${absolutePath}' is outside the sandbox cwd '${this.strictCwd}'. Strict path sandbox is enabled.`,
				absolutePath,
				"write",
			);
		}
		const reason = this.deniedWriteReason(absolutePath);
		if (reason) throw new PathGuardError(reason, absolutePath, "write");
	}

	/** Assert a read target is allowed, throwing with a clear message if not. */
	assertReadable(absolutePath: string): void {
		if (this.isOutsideStrictCwd(absolutePath)) {
			throw new PathGuardError(
				`Path '${absolutePath}' is outside the sandbox cwd '${this.strictCwd}'. Strict path sandbox is enabled.`,
				absolutePath,
				"read",
			);
		}
		const reason = this.deniedReadReason(absolutePath);
		if (reason) throw new PathGuardError(reason, absolutePath, "read");
	}
}

export class PathGuardError extends Error {
	readonly path: string;
	readonly operation: "read" | "write";
	constructor(message: string, path: string, operation: "read" | "write") {
		super(message);
		this.name = "PathGuardError";
		this.path = path;
		this.operation = operation;
	}
}

/** Detect paths inside any `.git/hooks` directory (any depth). */
function isGitHooksDir(absolutePath: string): boolean {
	const parts = absolutePath.split(sep);
	for (let i = 0; i < parts.length - 1; i++) {
		if (parts[i] === ".git" && parts[i + 1] === "hooks") return true;
	}
	return false;
}
