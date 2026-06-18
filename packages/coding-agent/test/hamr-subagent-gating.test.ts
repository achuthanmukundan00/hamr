import { describe, expect, it } from "vitest";
import type { ExtensionContext, ExtensionFactory, ToolDefinition } from "../src/core/extensions/types.ts";
import { registerSubagentTool } from "../src/hamr/subagents.ts";

type AnyTool = ToolDefinition<any, any, any>;

function collectSubagentTools(allowDispatch: (ctx: ExtensionContext) => boolean): Map<string, AnyTool> {
	const tools = new Map<string, AnyTool>();
	const pi = {
		registerTool: (tool: AnyTool) => tools.set(tool.name, tool),
	} as unknown as Parameters<ExtensionFactory>[0];
	registerSubagentTool(pi, () => [] as ExtensionFactory[], allowDispatch);
	return tools;
}

const relayCtx = { model: { provider: "relay" } } as unknown as ExtensionContext;

describe("Hamr subagent dispatch gating", () => {
	it("refuses delegate_subagent when dispatch is disallowed", async () => {
		const tools = collectSubagentTools(() => false);
		const result = await tools.get("delegate_subagent")!.execute(
			"call-1",
			{ task: "investigate", mode: "read_only" },
			undefined,
			undefined,
			relayCtx,
		);
		expect(result.isError).toBe(true);
		expect(result.content[0]).toMatchObject({ type: "text" });
		expect((result.content[0] as { text: string }).text).toContain("disabled for local/relay models");
	});

	it("refuses delegate_subagents when dispatch is disallowed", async () => {
		const tools = collectSubagentTools(() => false);
		const result = await tools.get("delegate_subagents")!.execute(
			"call-2",
			{ subtasks: [{ task: "investigate" }] },
			undefined,
			undefined,
			relayCtx,
		);
		expect(result.isError).toBe(true);
		expect((result.content[0] as { text: string }).text).toContain("disabled for local/relay models");
	});
});
