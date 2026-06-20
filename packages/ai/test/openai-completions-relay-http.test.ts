import { afterEach, describe, expect, it, vi } from "vitest";
import { streamOpenAICompletions } from "../src/providers/openai-completions.ts";
import type { Context, Model } from "../src/types.ts";

vi.mock("openai", () => {
	class FakeOpenAI {
		constructor() {
			throw new Error("Relay requests should bypass the OpenAI SDK");
		}
	}

	return { default: FakeOpenAI };
});

const relayModel: Model<"openai-completions"> = {
	id: "qwen3.6-35b-a3b",
	name: "Qwen 3.6 35B",
	api: "openai-completions",
	provider: "relay",
	baseUrl: "https://ai.watchyourtemper.com/v1",
	reasoning: true,
	input: ["text"],
	cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
	contextWindow: 131072,
	maxTokens: 16384,
	compat: {
		supportsUsageInStreaming: false,
		supportsStrictMode: false,
		maxTokensField: "max_tokens",
	},
};

const context: Context = {
	systemPrompt: "You are helpful.",
	messages: [{ role: "user", content: [{ type: "text", text: "Use the read tool on package.json" }], timestamp: 0 }],
	tools: [],
};

afterEach(() => {
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
});

function makeSseBody(lines: string[]): ReadableStream<Uint8Array> {
	const encoder = new TextEncoder();
	return new ReadableStream({
		start(controller) {
			for (const line of lines) {
				controller.enqueue(encoder.encode(line + "\n"));
			}
			controller.close();
		},
	});
}

describe("openai-completions relay raw HTTP path", () => {
	it("uses raw fetch for relay, streams SSE, and forwards custom access headers", async () => {
		const sseLines = [
			`data: ${JSON.stringify({ id: "chatcmpl-test", model: "Qwen3.6-35B-A3B-UD-IQ3_XXS.gguf", choices: [{ delta: { role: "assistant" }, finish_reason: null }] })}`,
			`data: ${JSON.stringify({ id: "chatcmpl-test", model: "Qwen3.6-35B-A3B-UD-IQ3_XXS.gguf", choices: [{ delta: { reasoning_content: "I should read package.json first." }, finish_reason: null }] })}`,
			`data: ${JSON.stringify({ id: "chatcmpl-test", model: "Qwen3.6-35B-A3B-UD-IQ3_XXS.gguf", choices: [{ delta: { tool_calls: [{ index: 0, id: "tool-1", function: { name: "read", arguments: "" } }] }, finish_reason: null }] })}`,
			`data: ${JSON.stringify({ id: "chatcmpl-test", model: "Qwen3.6-35B-A3B-UD-IQ3_XXS.gguf", choices: [{ delta: { tool_calls: [{ index: 0, function: { arguments: '{"path":"package.json"}' } }] }, finish_reason: null }] })}`,
			`data: ${JSON.stringify({ id: "chatcmpl-test", model: "Qwen3.6-35B-A3B-UD-IQ3_XXS.gguf", choices: [{ delta: {}, finish_reason: "tool_calls" }], usage: { prompt_tokens: 123, completion_tokens: 45, total_tokens: 168 } })}`,
			"data: [DONE]",
		];

		const fetchMock = vi.fn().mockResolvedValue(
			new Response(makeSseBody(sseLines), {
				status: 200,
				headers: new Headers({ "content-type": "text/event-stream", "x-request-id": "req-1" }),
			}),
		);
		vi.stubGlobal("fetch", fetchMock);

		const result = await streamOpenAICompletions(relayModel, context, {
			apiKey: "not-needed",
			maxTokens: 400,
			headers: {
				"CF-Access-Client-Id": "client-id",
				"CF-Access-Client-Secret": "client-secret",
			},
		}).result();

		expect(fetchMock).toHaveBeenCalledTimes(1);
		const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe("https://ai.watchyourtemper.com/v1/chat/completions");
		expect(init.method).toBe("POST");
		expect(init.headers).toMatchObject({
			Accept: "text/event-stream",
			"Content-Type": "application/json",
			"CF-Access-Client-Id": "client-id",
			"CF-Access-Client-Secret": "client-secret",
		});
		expect((init.headers as Record<string, string>).Authorization).toBeUndefined();

		const body = JSON.parse(String(init.body));
		expect(body.stream).toBe(true);
		expect(body.stream_options).toBeUndefined(); // supportsUsageInStreaming: false on this model
		expect(body.max_tokens).toBe(400);

		expect(result.stopReason).toBe("toolUse");
		expect(result.responseId).toBe("chatcmpl-test");
		expect(result.responseModel).toBe("Qwen3.6-35B-A3B-UD-IQ3_XXS.gguf");
		expect(result.usage.totalTokens).toBe(168);
		expect(result.content).toEqual([
			{
				type: "thinking",
				thinking: "I should read package.json first.",
				thinkingSignature: "reasoning_content",
			},
			{
				type: "toolCall",
				id: "tool-1",
				name: "read",
				arguments: { path: "package.json" },
			},
		]);
	});
});
