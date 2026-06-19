import { mkdir } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import type { BrowserContext, Page } from "playwright";
import { loadInstalledPlaywright } from "./deps.ts";
import { getDefaultBrowserProfileDir, getDefaultScreenshotDir } from "./paths.ts";
import { buildBrowserSnapshot } from "./snapshot.ts";
import { locatorForTarget, normalizeScrollAmount } from "./targeting.ts";

export interface BrowserState {
	profileDir: string;
	screenshotDir: string;
	channel?: string;
	headless: boolean;
}

export interface BrowserLaunchOptions {
	url?: string;
	ensureDependencies?: () => Promise<void>;
}

function conciseError(error: unknown, fallback: string): Error {
	if (error instanceof Error && error.message.trim()) {
		return new Error(error.message.split("\n")[0]);
	}
	return new Error(fallback);
}

function isTruthy(value: string | undefined): boolean {
	return value === "1" || value?.toLowerCase() === "true" || value?.toLowerCase() === "yes";
}

function resolveState(): BrowserState {
	const home = homedir();
	const channel = process.env.HAMR_BROWSER_CHANNEL || (isTruthy(process.env.HAMR_BROWSER_USE_CHROME) ? "chrome" : undefined);
	return {
		profileDir: process.env.HAMR_BROWSER_PROFILE_DIR || getDefaultBrowserProfileDir(process.platform, home, process.env),
		screenshotDir: process.env.HAMR_BROWSER_SCREENSHOT_DIR || getDefaultScreenshotDir(home),
		channel,
		headless: isTruthy(process.env.HAMR_BROWSER_HEADLESS) ? true : false,
	};
}

export class HamrBrowserController {
	private context: BrowserContext | undefined;
	private page: Page | undefined;
	private state: BrowserState = resolveState();

	getState(): BrowserState {
		this.state = resolveState();
		return this.state;
	}

	isOpen(): boolean {
		return !!this.context;
	}

	async launch(options: BrowserLaunchOptions = {}): Promise<string> {
		try {
			if (this.context && this.page) {
				if (options.url) await this.openUrl(options.url);
				return `Hamr Browser already open. Profile: ${this.state.profileDir}`;
			}

			this.state = resolveState();
			await mkdir(this.state.profileDir, { recursive: true });
			let playwright = loadInstalledPlaywright();
			if (!playwright && options.ensureDependencies) {
				await options.ensureDependencies();
				playwright = loadInstalledPlaywright();
			}
			if (!playwright) {
				throw new Error("Playwright is not installed for Hamr Browser");
			}
			this.context = await playwright.chromium.launchPersistentContext(this.state.profileDir, {
				headless: this.state.headless,
				channel: this.state.channel,
				viewport: null,
			});
			this.page = this.context.pages()[0] ?? (await this.context.newPage());
			if (options.url) {
				await this.openUrl(options.url, options.ensureDependencies);
			}
			return `Opening isolated Hamr Browser. Your normal browser profile is untouched. Profile: ${this.state.profileDir}`;
		} catch (error) {
			throw conciseError(error, "Failed to launch Hamr Browser");
		}
	}

	async openUrl(url: string, ensureDependencies?: () => Promise<void>): Promise<string> {
		const page = await this.requirePage(ensureDependencies);
		try {
			const parsed = new URL(url);
			if (parsed.protocol !== "http:" && parsed.protocol !== "https:" && parsed.protocol !== "file:") {
				throw new Error("URL must use http, https, or file");
			}
			await page.goto(parsed.toString(), { waitUntil: "domcontentloaded" });
			return `Opened ${page.url()}`;
		} catch (error) {
			throw conciseError(error, `Failed to open URL: ${url}`);
		}
	}

	async snapshot(): Promise<string> {
		const page = await this.requirePage();
		try {
			return await buildBrowserSnapshot(page);
		} catch (error) {
			throw conciseError(error, "Failed to snapshot page");
		}
	}

	async click(target: string): Promise<string> {
		const page = await this.requirePage();
		try {
			const locator = locatorForTarget(page, target);
			await locator.click({ timeout: 5_000 });
			return `Clicked ${target}`;
		} catch (error) {
			throw conciseError(error, `Could not click target: ${target}`);
		}
	}

	async type(target: string, text: string): Promise<string> {
		const page = await this.requirePage();
		try {
			const locator = locatorForTarget(page, target);
			try {
				await locator.fill(text, { timeout: 5_000 });
			} catch {
				await locator.click({ timeout: 5_000 });
				await page.keyboard.type(text);
			}
			return `Typed into ${target}`;
		} catch (error) {
			throw conciseError(error, `Could not type into target: ${target}`);
		}
	}

	async press(key: string): Promise<string> {
		const page = await this.requirePage();
		try {
			await page.keyboard.press(key);
			return `Pressed ${key}`;
		} catch (error) {
			throw conciseError(error, `Could not press key: ${key}`);
		}
	}

	async scroll(directionOrPixels: string | number): Promise<string> {
		const page = await this.requirePage();
		try {
			const { x, y } = normalizeScrollAmount(directionOrPixels);
			await page.mouse.wheel(x, y);
			return `Scrolled x=${x} y=${y}`;
		} catch (error) {
			throw conciseError(error, "Could not scroll page");
		}
	}

	async wait(ms: number): Promise<string> {
		const page = await this.requirePage();
		const boundedMs = Math.max(0, Math.min(ms, 60_000));
		await page.waitForTimeout(boundedMs);
		return `Waited ${boundedMs}ms`;
	}

	async screenshot(): Promise<string> {
		const page = await this.requirePage();
		try {
			await mkdir(this.state.screenshotDir, { recursive: true });
			const stamp = new Date().toISOString().replace(/[:.]/g, "-");
			const path = join(this.state.screenshotDir, `hamr-browser-${stamp}.png`);
			await page.screenshot({ path, fullPage: true });
			return path;
		} catch (error) {
			throw conciseError(error, "Could not save screenshot");
		}
	}

	async close(): Promise<string> {
		if (!this.context) {
			this.page = undefined;
			return "Hamr Browser is already closed";
		}
		try {
			await this.context.close();
			return "Closed Hamr Browser";
		} catch (error) {
			throw conciseError(error, "Failed to close Hamr Browser");
		} finally {
			this.context = undefined;
			this.page = undefined;
		}
	}

	private async requirePage(ensureDependencies?: () => Promise<void>): Promise<Page> {
		if (!this.context || !this.page) {
			await this.launch({ ensureDependencies });
		}
		if (!this.page) {
			throw new Error("Hamr Browser is not open");
		}
		return this.page;
	}
}
