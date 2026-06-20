import { describe, expect, it } from "vitest";
import { convertMessages } from "../src/providers/openai-completions.ts";
import type { Context, Model } from "../src/types.ts";

const relayModel: Model<"openai-completions"> = {
	id: "devstral-2-24b",
	name: "Devstral 2 24B",
	api: "openai-completions",
	provider: "relay",
	baseUrl: "https://ai.watchyourtemper.com/v1",
	reasoning: true,
	input: ["text"],
	cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
	contextWindow: 32768,
	maxTokens: 16384,
	compat: {
		supportsDeveloperRole: false,
		supportsUsageInStreaming: false,
		supportsStrictMode: false,
		maxTokensField: "max_tokens",
	},
};

describe("openai-completions consecutive user turns", () => {
	it("passes through adjacent user messages unmerged", () => {
		const context: Context = {
			systemPrompt: "You are helpful.",
			messages: [
				{
					role: "user",
					content: "Auto-retrieved context from prior sessions.",
					timestamp: 0,
				},
				{
					role: "user",
					content: [{ type: "text", text: "Read package.json and tell me the package name" }],
					timestamp: 1,
				},
			],
			tools: [],
		};

		const messages = convertMessages(relayModel, context, relayModel.compat);

		expect(messages).toHaveLength(3);
		expect(messages[0]).toEqual({
			role: "system",
			content: "You are helpful.",
		});
		expect(messages[1]).toEqual({
			role: "user",
			content: "Auto-retrieved context from prior sessions.",
		});
		expect(messages[2]).toEqual({
			role: "user",
			content: [{ type: "text", text: "Read package.json and tell me the package name" }],
		});
	});
});
