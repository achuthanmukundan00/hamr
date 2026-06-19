import { describe, expect, it } from "vitest";
import type { ExtensionFactory, ToolDefinition } from "../src/core/extensions/types.ts";
import { createHamrSubagentsExtension } from "../src/hamr/extensions/subagents.ts";

type AnyTool = ToolDefinition<any, any, any>;

/** Run the subagents factory at a given depth and return the tools it registered. */
async function toolsAtDepth(depth: number): Promise<Map<string, AnyTool>> {
	const tools = new Map<string, AnyTool>();
	const pi = {
		registerTool: (tool: AnyTool) => tools.set(tool.name, tool),
		registerShortcut: () => {},
	} as unknown as Parameters<ExtensionFactory>[0];
	await createHamrSubagentsExtension(() => [] as ExtensionFactory[], depth)(pi);
	return tools;
}

describe("Hamr subagent depth bound", () => {
	it("registers delegate_subagents below the depth bound", async () => {
		expect((await toolsAtDepth(0)).has("delegate_subagents")).toBe(true);
		expect((await toolsAtDepth(2)).has("delegate_subagents")).toBe(true);
	});

	it("omits the tool at the depth bound so recursion stops (leaf)", async () => {
		expect((await toolsAtDepth(3)).has("delegate_subagents")).toBe(false);
	});
});
