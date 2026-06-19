import { join } from "node:path";

export type PlatformName = NodeJS.Platform;

export type BrowserPathEnv = Partial<Record<"LOCALAPPDATA" | "APPDATA", string | undefined>>;

export function getDefaultBrowserProfileDir(
	platform: PlatformName = process.platform,
	homeDir: string,
	env: BrowserPathEnv = process.env,
): string {
	if (platform === "win32") {
		const appDataRoot = env.LOCALAPPDATA || (homeDir ? join(homeDir, "AppData", "Local") : env.APPDATA);
		if (!appDataRoot) {
			return join(homeDir, ".hamr", "browser-profile");
		}
		return join(appDataRoot, "Hamr", "browser-profile");
	}

	return join(homeDir, ".hamr", "browser-profile");
}

export function getDefaultScreenshotDir(homeDir: string): string {
	return join(homeDir, ".hamr", "browser-artifacts", "screenshots");
}
