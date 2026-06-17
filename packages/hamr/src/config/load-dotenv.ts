/**
 * Auto-load .env files into process.env.
 *
 * Semantics (matches Claude Code):
 *   1. Reads ~/workspace/.env  (user-level workspace env)
 *   2. Reads <cwd>/.env          (project-level env)
 *   3. NEVER overwrites an existing process.env value
 *   4. Skips if HAMR_SKIP_DOTENV=1
 *
 * Parses shell-style `export KEY=VALUE` lines.
 * Calls are idempotent — safe to invoke multiple times.
 */

import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const ENV_LINE_RE = /^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/;

/**
 * Parse a single .env line, stripping surrounding quotes.
 */
function parseEnvLine(line: string): [string, string] | null {
	const trimmed = line.trim();
	if (!trimmed || trimmed.startsWith("#")) return null;
	const m = trimmed.match(ENV_LINE_RE);
	if (!m) return null;
	let value = m[2].trim();
	// Strip matching quotes
	if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
		value = value.slice(1, -1);
	}
	return [m[1], value];
}

/**
 * Load a single .env file into process.env (no overwrite).
 */
function loadEnvFile(filePath: string): number {
	if (!existsSync(filePath)) return 0;
	const content = readFileSync(filePath, "utf-8");
	let loaded = 0;
	for (const line of content.split("\n")) {
		const pair = parseEnvLine(line);
		if (pair) {
			const [key, value] = pair;
			// ??=  →  existing vars take priority
			if (process.env[key] === undefined) {
				process.env[key] = value;
				loaded++;
			}
		}
	}
	return loaded;
}

/**
 * Auto-load .env files into process.env.
 *
 * Skips loading entirely when the env var HAMR_SKIP_DOTENV is set to a truthy
 * value (1, true, yes).
 *
 * Files searched (in order, later = lower priority):
 *   - ~/workspace/.env
 *   - <cwd>/.env
 *
 * Existing process.env values are NEVER overwritten.
 *
 * Returns the total number of variables loaded.
 */
export function loadDotenv(): number {
	if (process.env.HAMR_SKIP_DOTENV) {
		const skip = process.env.HAMR_SKIP_DOTENV.toLowerCase();
		if (skip === "1" || skip === "true" || skip === "yes") {
			return 0;
		}
	}

	let total = 0;

	// 1. User-level .env (home directory)
	const userHomeEnv = join(homedir(), ".env");
	total += loadEnvFile(userHomeEnv);

	// 2. User-level workspace env
	const homeEnv = join(homedir(), "workspace", ".env");
	total += loadEnvFile(homeEnv);

	// 3. Project-level .env in cwd
	const cwdEnv = join(process.cwd(), ".env");
	total += loadEnvFile(cwdEnv);

	if (total > 0) {
		// Use console.warn so it goes to stderr (no interference with stdout)
		console.warn(`[hamr] Loaded ${total} env var(s) from .env files`);
	}

	return total;
}
