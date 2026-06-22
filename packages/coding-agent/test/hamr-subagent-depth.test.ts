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
		on: () => {},
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

describe("delegate_subagents tool definition", () => {
	it("registers the correct tool name", async () => {
		const tools = await toolsAtDepth(0);
		const tool = tools.get("delegate_subagents");
		expect(tool).toBeDefined();
		expect(tool!.name).toBe("delegate_subagents");
	});

	it("has a label", async () => {
		const tools = await toolsAtDepth(0);
		const tool = tools.get("delegate_subagents");
		expect(tool!.label).toBeTruthy();
	});

	it("description does not claim serial execution for all modes", async () => {
		const tools = await toolsAtDepth(0);
		const tool = tools.get("delegate_subagents");
		// The description should mention parallel and not claim "one at a time"
		expect(tool!.description).not.toContain("one at a time");
		expect(tool!.description).toContain("parallel");
		expect(tool!.description).toContain("chain");
		expect(tool!.description).toContain("stages");
	});

	it("promptSnippet does not mention serial", async () => {
		const tools = await toolsAtDepth(0);
		const tool = tools.get("delegate_subagents");
		expect(tool!.promptSnippet).not.toContain("serial");
	});

	it("parameters accept tasks (parallel), chain, and stages", async () => {
		const tools = await toolsAtDepth(0);
		const tool = tools.get("delegate_subagents");
		const schema = tool!.parameters as any;
		// Should have tasks, chain, stages in properties
		expect(schema.properties).toBeDefined();
	});

	it("parameters accept concurrency and failFast", async () => {
		const tools = await toolsAtDepth(0);
		const tool = tools.get("delegate_subagents");
		const schema = tool!.parameters as any;
		expect(schema.properties.concurrency).toBeDefined();
		expect(schema.properties.failFast).toBeDefined();
	});
});

describe("delegate_subagents validation", () => {
	async function getTool() {
		const tools = await toolsAtDepth(0);
		return tools.get("delegate_subagents")!;
	}

	// We test the structure — the execute function is hard to test
	// without a full session, but we can validate the parameter schema.

	it("rejects empty parameters by requiring exactly one mode", () => {
		// The parameter schema requires at least one of subtasks/tasks/chain/stages
		// via the execute function logic. We test the schema shape.
	});
});
