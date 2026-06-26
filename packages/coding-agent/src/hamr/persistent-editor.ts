import type { Component, TUI } from "@hamr/tui";
import { Key, sliceWithWidth } from "@hamr/tui";
import type { ExtensionAPI, ExtensionContext } from "../core/extensions/types.ts";
import type { ReadonlyFooterDataProvider } from "../core/footer-data-provider.ts";
import type { Theme } from "../modes/interactive/theme/theme.ts";

class PersistentFooterComponent implements Component {
	private ctx: ExtensionContext;
	private theme: Theme;
	private footerData: ReadonlyFooterDataProvider;

	constructor(ctx: ExtensionContext, theme: Theme, footerData: ReadonlyFooterDataProvider) {
		this.ctx = ctx;
		this.theme = theme;
		this.footerData = footerData;
	}

	invalidate(): void {}
	dispose(): void {}

	render(width: number): string[] {
		const lines: string[] = [];
		const th = this.theme;

		const editorText = this.ctx.ui.getEditorText();
		const promptPrefix = "persistent> ";
		const promptStr =
			editorText.length > 0 ? `${promptPrefix}${editorText}` : `${promptPrefix}${th.fg("dim", "type a message...")}`;
		lines.push(promptStr.length > width ? promptStr.slice(0, width) : promptStr);

		const cwd = this.ctx.cwd;
		const branch = this.footerData.getGitBranch();
		const pwdStr = branch ? `${cwd} (${branch})` : cwd;
		lines.push(th.fg("dim", pwdStr.length > width ? pwdStr.slice(0, width) : pwdStr));

		// Model name with brand color (model-aware coloring)
		const model = this.ctx.model;
		const modelName = model?.id ?? "no-model";
		const modelColorAnsi = model ? th.modelColor(model.provider, model.id) : th.getFgAnsi("dim");
		const modelReset = "\x1b[39m";
		let statsLine = `${modelColorAnsi}${modelName}${modelReset}`;
		try {
			const usage = this.ctx.getContextUsage();
			if (usage) {
				const pct = usage.percent !== null ? `${usage.percent.toFixed(1)}%` : "?";
				const windowStr =
					usage.contextWindow > 0
						? usage.contextWindow < 1000
							? String(usage.contextWindow)
							: `${Math.round(usage.contextWindow / 1000)}k`
						: "?";
				statsLine = `${modelColorAnsi}${modelName}${modelReset} ${th.fg("dim", `| ${pct}/${windowStr}`)}`;
			}
		} catch {}
		lines.push(sliceWithWidth(statsLine, 0, width, true).text);

		const extensionStatuses = this.footerData.getExtensionStatuses();
		if (extensionStatuses.size > 0) {
			const sortedStatuses = Array.from(extensionStatuses.entries())
				.sort((a, b) => a[0].localeCompare(b[0]))
				.map((entry) => entry[1]);
			const statusLine = sortedStatuses.join(" ");
			lines.push(sliceWithWidth(statusLine, 0, width, true).text);
		}

		return lines;
	}
}

export function createPersistentEditorExtension(): (pi: ExtensionAPI) => void {
	let persistentEnabled = false;

	return function persistentEditorExtension(pi: ExtensionAPI): void {
		function togglePersistentMode(ctx: ExtensionContext): void {
			persistentEnabled = !persistentEnabled;
			if (persistentEnabled) {
				enablePersistentMode(ctx);
			} else {
				disablePersistentMode(ctx);
			}
		}

		function enablePersistentMode(ctx: ExtensionContext): void {
			ctx.ui.setFooter(
				(_tui: TUI, theme: Theme, footerData: ReadonlyFooterDataProvider) =>
					new PersistentFooterComponent(ctx, theme, footerData),
			);
			ctx.ui.notify(`Persistent editor ${ctx.ui.theme.fg("success", "enabled")} (Shift+Ctrl+U to toggle)`, "info");
		}

		function disablePersistentMode(ctx: ExtensionContext): void {
			ctx.ui.setFooter(undefined);
			ctx.ui.notify(`Persistent editor ${ctx.ui.theme.fg("muted", "disabled")}`, "info");
		}

		pi.registerShortcut(Key.shiftCtrl("u"), {
			description: "Toggle persistent editor mode (keep editor open at bottom)",
			handler: async (ctx: ExtensionContext): Promise<void> => {
				togglePersistentMode(ctx);
			},
		});

		pi.registerCommand("persistent-editor", {
			description: "Toggle persistent editor mode (keep editor open at bottom)",
			handler: async (_args: string, ctx: ExtensionContext): Promise<void> => {
				togglePersistentMode(ctx);
			},
		});
	};
}
