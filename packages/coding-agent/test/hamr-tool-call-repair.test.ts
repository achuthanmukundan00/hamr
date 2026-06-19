import type { AssistantMessage } from "@hamr/ai";
import { describe, expect, it } from "vitest";
import type { ExtensionContext } from "../src/core/extensions/types.ts";
import { repairLocalToolCalls } from "../src/hamr/repair.ts";

function assistant(content: AssistantMessage["content"], model = "qwen3.6-35b-a3b"): AssistantMessage {
	return {
		role: "assistant",
		content,
		api: "openai-completions",
		provider: "relay",
		model,
		usage: {
			input: 0,
			output: 0,
			cacheRead: 0,
			cacheWrite: 0,
			totalTokens: 0,
			cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
		},
		stopReason: "stop",
		timestamp: Date.now(),
	};
}

const relayCtx = {
	model: { provider: "relay", id: "qwen3.6-35b-a3b" },
} as unknown as ExtensionContext;

describe("Hamr local tool-call repair", () => {
	it("repairs Qwen XML tool calls emitted only in reasoning content", () => {
		const message = assistant([
			{
				type: "thinking",
				thinking:
					"<tool_call>\n<function=bash>\n<parameter=command>git status --short</parameter>\n</function>\n</tool_call>",
				thinkingSignature: "reasoning_content",
			},
		]);

		const repaired = repairLocalToolCalls(message, relayCtx);

		expect(repaired?.stopReason).toBe("toolUse");
		expect(repaired?.content).toEqual([
			{
				type: "thinking",
				thinking:
					"<tool_call>\n<function=bash>\n<parameter=command>git status --short</parameter>\n</function>\n</tool_call>",
			},
			{
				type: "toolCall",
				id: expect.any(String),
				name: "bash",
				arguments: { command: "git status --short" },
			},
		]);
		expect(repaired?.diagnostics?.some((diagnostic) => diagnostic.type === "hamr.tool_call_repair")).toBe(true);
	});

	it("does not repair messages with neither visible text nor reasoning", () => {
		expect(repairLocalToolCalls(assistant([]), relayCtx)).toBeUndefined();
	});
});
