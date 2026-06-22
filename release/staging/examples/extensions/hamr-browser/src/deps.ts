import { createRequire } from "node:module";
import { homedir } from "node:os";
import { join } from "node:path";
import type { BrowserType } from "playwright";

export const PLAYWRIGHT_VERSION = "^1.57.0";

export interface PlaywrightModule {
	chromium: BrowserType;
}

export function getHamrAgentDir(env: NodeJS.ProcessEnv = process.env): string {
	return env.HAMR_CODING_AGENT_DIR || join(homedir(), ".hamr", "agent");
}

export function getBrowserDepsDir(env: NodeJS.ProcessEnv = process.env): string {
	return env.HAMR_BROWSER_DEPS_DIR || join(getHamrAgentDir(env), "browser-deps");
}

export function getBrowserDepsInstallCommand(depsDir = getBrowserDepsDir()): string[] {
	return ["npm", "install", "--prefix", depsDir, `playwright@${PLAYWRIGHT_VERSION}`];
}

export function loadInstalledPlaywright(depsDir = getBrowserDepsDir()): PlaywrightModule | undefined {
	const candidateRequires = [createRequire(import.meta.url), createRequire(join(depsDir, "package.json"))];

	for (const requireFrom of candidateRequires) {
		try {
			return requireFrom("playwright") as PlaywrightModule;
		} catch (error) {
			const code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : undefined;
			if (code !== "MODULE_NOT_FOUND") {
				throw error;
			}
		}
	}

	return undefined;
}
