import type { ExtensionFactory } from "../../core/extensions/types.ts";

const READ_ONLY_TOOLS = new Set(["read", "grep", "find", "ls"]);

export const hamrReadLoopGuardExtension: ExtensionFactory = (pi) => {
	let readOnlyCounter = 0;
	let lastNudge = 0;

	pi.on("turn_end", (event, ctx) => {
		const toolResults = event.toolResults ?? [];
		if (toolResults.length === 0) {
			readOnlyCounter = 0;
			return;
		}

		const allReadOnly = toolResults.every((t) => READ_ONLY_TOOLS.has(t.toolName));
		if (allReadOnly) {
			readOnlyCounter++;
			if (readOnlyCounter >= 5 && Date.now() - lastNudge > 30000) {
				lastNudge = Date.now();
				if (!ctx.isIdle()) {
					pi.sendUserMessage(
						`(You've done ${readOnlyCounter} consecutive read-only operations. If you have enough context, consider editing or writing.)`,
						{ deliverAs: "steer" },
					);
				}
			}
		} else {
			readOnlyCounter = 0;
		}
	});
};
