import { Key } from "@hamr/tui";
import type { ExtensionFactory } from "../../core/extensions/types.ts";
import { isCloudProvider, loadHamrStartupConfig } from "../startup-config.ts";
import { hasSubagentRuns, registerSubagentTool, showAgentDashboard, showIdleAgentDashboard } from "../subagents.ts";

/**
 * Subagents extension: the `delegate_subagent(s)` tools + the agent dashboard.
 * Dispatch is gated behind the active model's provider cloud flag so local/relay
 * models can't fan out parallel work onto a single backend.
 *
 * `getChildExtensions` supplies the extension set a spawned child session loads
 * (passed lazily to avoid an import cycle with the default-extension array).
 *
 * NOTE: spawned subagents are currently detached in-memory sessions rather than
 * nodes in the parent session tree — see issue #4.
 */
export function createHamrSubagentsExtension(getChildExtensions: () => ExtensionFactory[]): ExtensionFactory {
	return async (pi) => {
		const config = loadHamrStartupConfig(process.cwd());
		registerSubagentTool(pi, getChildExtensions, (ctx) =>
			ctx.model ? isCloudProvider(config, ctx.model.provider) : false,
		);

		// Keep pi's Ctrl+O for tool expansion; use Shift+Ctrl+O for the dashboard.
		pi.registerShortcut(Key.shiftCtrl("o"), {
			description: "Open agent dashboard (or slash-command dashboard if no subagents are running)",
			handler: async (ctx) => {
				if (ctx.mode !== "tui") return;
				if (hasSubagentRuns()) {
					const result = await showAgentDashboard(pi, ctx);
					if (result.action === "retry") {
						ctx.ui.notify("Retry not yet implemented", "warning");
					}
				} else {
					await showIdleAgentDashboard(pi, ctx);
				}
			},
		});
	};
}
