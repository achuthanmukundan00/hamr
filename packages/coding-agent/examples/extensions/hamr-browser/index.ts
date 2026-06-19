import { Type } from "typebox";
import { getBrowserDepsDir, getBrowserDepsInstallCommand, loadInstalledPlaywright } from "./src/deps.ts";

type ToolContent = { type: "text"; text: string };
type ToolResult = { content: ToolContent[]; details?: Record<string, unknown> };
type ToolUpdate = (result: ToolResult) => void;
type BrowserUi = {
	notify(message: string, level: "info" | "warning" | "error"): void;
	confirm?(title: string, message: string): Promise<boolean>;
};
type ToolContext = { hasUI?: boolean; ui: BrowserUi };
type ToolDefinition = {
	name: string;
	label: string;
	description: string;
	promptSnippet?: string;
	promptGuidelines?: string[];
	parameters: unknown;
	execute(
		toolCallId: string,
		params: any,
		signal: AbortSignal | undefined,
		onUpdate: ToolUpdate | undefined,
		ctx: ToolContext,
	): Promise<ToolResult> | ToolResult;
};
type CommandContext = { hasUI?: boolean; ui: BrowserUi };
type ExtensionAPI = {
	registerCommand(
		name: string,
		options: { description?: string; handler(args: string, ctx: CommandContext): Promise<void> | void },
	): void;
	registerTool(definition: ToolDefinition): void;
	on(event: "session_shutdown", handler: () => Promise<void> | void): void;
	exec(
		command: string,
		args: string[],
		options?: { signal?: AbortSignal; timeout?: number },
	): Promise<{ code: number | null; stdout: string; stderr: string }>;
};
import { HamrBrowserController } from "./src/browser-controller.ts";

const controller = new HamrBrowserController();

function textResult(text: string, details: Record<string, unknown> = {}) {
	return {
		content: [{ type: "text" as const, text }],
		details,
	};
}

function checkAbort(signal: AbortSignal | undefined): void {
	if (signal?.aborted) {
		throw new Error("Cancelled");
	}
}

function shellQuote(value: string): string {
	return /[\s"']/.test(value) ? JSON.stringify(value) : value;
}

export default function hamrBrowserExtension(hamr: ExtensionAPI) {
	async function ensureBrowserDependencies(
		ctx: ToolContext | CommandContext,
		signal?: AbortSignal,
		onUpdate?: ToolUpdate,
	): Promise<void> {
		if (loadInstalledPlaywright()) return;

		const depsDir = getBrowserDepsDir();
		const installCommand = getBrowserDepsInstallCommand(depsDir);
		const displayCommand = installCommand.map(shellQuote).join(" ");
		const message = `Hamr Browser needs Playwright before it can open a browser. Install it into ${depsDir}?\n\nCommand: ${displayCommand}`;
		if (!ctx.hasUI || !ctx.ui.confirm) {
			throw new Error(`Playwright is not installed. Run: ${displayCommand}`);
		}

		const ok = await ctx.ui.confirm("Install Hamr Browser dependencies?", message);
		if (!ok) {
			throw new Error("Hamr Browser dependency install cancelled");
		}

		onUpdate?.(textResult(`Installing Hamr Browser dependencies with: ${displayCommand}`, { depsDir }));
		const [command, ...args] = installCommand;
		const result = await hamr.exec(command, args, { signal, timeout: 10 * 60 * 1000 });
		if (result.code !== 0) {
			const detail = (result.stderr || result.stdout || `exit code ${result.code}`).trim().split("\n").slice(-1)[0];
			throw new Error(`Failed to install Playwright: ${detail}`);
		}
		if (!loadInstalledPlaywright()) {
			throw new Error(`Playwright install finished, but module was not found in ${depsDir}`);
		}
	}

	hamr.registerCommand("browser", {
		description: "Launch the isolated visible Hamr Browser window",
		handler: async (args, ctx) => {
			const url = args.trim() || undefined;
			const message = await controller.launch({ url, ensureDependencies: () => ensureBrowserDependencies(ctx) });
			ctx.ui.notify(message, "info");
		},
	});

	hamr.registerTool({
		name: "browser_launch",
		label: "Browser Launch",
		description: "Launch Hamr's isolated visible browser window. Does not use the user's normal browser profile.",
		promptSnippet: "Launch Hamr's isolated visible browser window",
		promptGuidelines: [
			"Use browser_launch before other browser tools when a visible Hamr-controlled browser is needed.",
			"Hamr Browser uses an isolated profile; do not claim it has access to the user's normal browser session.",
		],
		parameters: Type.Object({
			url: Type.Optional(Type.String({ description: "Optional http, https, or file URL to open after launch" })),
		}),
		async execute(_toolCallId, params, signal, onUpdate, ctx) {
			checkAbort(signal);
			const message = await controller.launch({
				url: params.url,
				ensureDependencies: () => ensureBrowserDependencies(ctx, signal, onUpdate),
			});
			if (ctx.hasUI) ctx.ui.notify(message, "info");
			return textResult(message, { ...controller.getState() });
		},
	});

	hamr.registerTool({
		name: "browser_open_url",
		label: "Browser Open URL",
		description: "Open a URL in Hamr Browser. Launches the isolated visible browser if needed.",
		promptSnippet: "Open a URL in Hamr Browser",
		parameters: Type.Object({
			url: Type.String({ description: "http, https, or file URL to open" }),
		}),
		async execute(_toolCallId, params, signal, onUpdate, ctx) {
			checkAbort(signal);
			return textResult(await controller.openUrl(params.url, () => ensureBrowserDependencies(ctx, signal, onUpdate)), {
				url: params.url,
			});
		},
	});

	hamr.registerTool({
		name: "browser_snapshot",
		label: "Browser Snapshot",
		description: "Return cheap text, DOM, and accessibility-ish state for the current Hamr Browser page.",
		promptSnippet: "Snapshot current Hamr Browser page text and interactive elements",
		parameters: Type.Object({}),
		async execute(_toolCallId, _params, signal, onUpdate, ctx) {
			checkAbort(signal);
			await controller.launch({ ensureDependencies: () => ensureBrowserDependencies(ctx, signal, onUpdate) });
			const snapshot = await controller.snapshot();
			return textResult(snapshot, { chars: snapshot.length });
		},
	});

	hamr.registerTool({
		name: "browser_click",
		label: "Browser Click",
		description: "Click a target in Hamr Browser. Targets can be css=selector, text=label, role=button:Name, or plain visible text.",
		promptSnippet: "Click a target in Hamr Browser",
		parameters: Type.Object({
			target: Type.String({ description: "css=selector, text=label, role=button:Name, CSS selector, or visible text" }),
		}),
		async execute(_toolCallId, params, signal, onUpdate, ctx) {
			checkAbort(signal);
			await controller.launch({ ensureDependencies: () => ensureBrowserDependencies(ctx, signal, onUpdate) });
			return textResult(await controller.click(params.target), { target: params.target });
		},
	});

	hamr.registerTool({
		name: "browser_type",
		label: "Browser Type",
		description: "Fill or type text into a target in Hamr Browser. Targets can be css=selector, text=label, role=textbox:Name, or plain visible text.",
		promptSnippet: "Type text into a target in Hamr Browser",
		parameters: Type.Object({
			target: Type.String({ description: "css=selector, text=label, role=textbox:Name, CSS selector, or visible text" }),
			text: Type.String({ description: "Text to enter" }),
		}),
		async execute(_toolCallId, params, signal, onUpdate, ctx) {
			checkAbort(signal);
			await controller.launch({ ensureDependencies: () => ensureBrowserDependencies(ctx, signal, onUpdate) });
			return textResult(await controller.type(params.target, params.text), { target: params.target });
		},
	});

	hamr.registerTool({
		name: "browser_press",
		label: "Browser Press",
		description: "Press a keyboard key in Hamr Browser, such as Enter, Escape, Tab, or Meta+L.",
		promptSnippet: "Press a key in Hamr Browser",
		parameters: Type.Object({
			key: Type.String({ description: "Playwright key name or shortcut, e.g. Enter, Tab, Escape, Meta+L" }),
		}),
		async execute(_toolCallId, params, signal, onUpdate, ctx) {
			checkAbort(signal);
			await controller.launch({ ensureDependencies: () => ensureBrowserDependencies(ctx, signal, onUpdate) });
			return textResult(await controller.press(params.key), { key: params.key });
		},
	});

	hamr.registerTool({
		name: "browser_scroll",
		label: "Browser Scroll",
		description: "Scroll Hamr Browser by direction (up/down/left/right) or a numeric pixel count.",
		promptSnippet: "Scroll the current Hamr Browser page",
		parameters: Type.Object({
			direction_or_pixels: Type.String({ description: "up, down, left, right, or a pixel count like 800 or -300" }),
		}),
		async execute(_toolCallId, params, signal, onUpdate, ctx) {
			checkAbort(signal);
			await controller.launch({ ensureDependencies: () => ensureBrowserDependencies(ctx, signal, onUpdate) });
			return textResult(await controller.scroll(params.direction_or_pixels), {
				direction_or_pixels: params.direction_or_pixels,
			});
		},
	});

	hamr.registerTool({
		name: "browser_wait",
		label: "Browser Wait",
		description: "Wait in Hamr Browser for the given milliseconds, capped at 60000ms.",
		promptSnippet: "Wait for page activity in Hamr Browser",
		parameters: Type.Object({
			ms: Type.Number({ description: "Milliseconds to wait, capped at 60000" }),
		}),
		async execute(_toolCallId, params, signal, onUpdate, ctx) {
			checkAbort(signal);
			await controller.launch({ ensureDependencies: () => ensureBrowserDependencies(ctx, signal, onUpdate) });
			return textResult(await controller.wait(params.ms), { ms: params.ms });
		},
	});

	hamr.registerTool({
		name: "browser_screenshot",
		label: "Browser Screenshot",
		description: "Save a full-page screenshot artifact from Hamr Browser and return the file path.",
		promptSnippet: "Save a Hamr Browser screenshot artifact",
		parameters: Type.Object({}),
		async execute(_toolCallId, _params, signal, onUpdate, ctx) {
			checkAbort(signal);
			await controller.launch({ ensureDependencies: () => ensureBrowserDependencies(ctx, signal, onUpdate) });
			const path = await controller.screenshot();
			return textResult(`Saved screenshot: ${path}`, { path });
		},
	});

	hamr.registerTool({
		name: "browser_close",
		label: "Browser Close",
		description: "Close Hamr Browser and release its Playwright context.",
		promptSnippet: "Close Hamr Browser",
		parameters: Type.Object({}),
		async execute(_toolCallId, _params, signal) {
			checkAbort(signal);
			return textResult(await controller.close());
		},
	});

	hamr.on("session_shutdown", async () => {
		await controller.close();
	});
}
