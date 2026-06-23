import { getSupportedThinkingLevels } from "@hamr/ai";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { discoverRelayModels } from "../src/hamr/providers/relay-provider.ts";
import { buildHamrProviderRegistrations, type HamrStartupConfig, isCloudProvider } from "../src/hamr/startup-config.ts";

function relayConfigWith(model: Record<string, unknown>): HamrStartupConfig {
	return {
		providers: {
			relay: {
				enabled: true,
				name: "Relay",
				compatibility: "openai-compatible",
				base_url: "http://127.0.0.1:1234/v1",
				models: [{ id: "test-model", ...model }],
			},
		},
		sourcePaths: [],
	};
}

async function supportedLevels(model: Record<string, unknown>): Promise<string[]> {
	const registrations = await buildHamrProviderRegistrations(relayConfigWith(model));
	const providerModel = registrations[0]?.config.models?.[0];
	// getSupportedThinkingLevels only reads `reasoning` and `thinkingLevelMap`.
	return getSupportedThinkingLevels(providerModel as Parameters<typeof getSupportedThinkingLevels>[0]);
}

describe("Hamr relay thinking-level mapping", () => {
	it("exposes exactly the levels the relay advertises", async () => {
		const levels = await supportedLevels({
			supports_thinking: true,
			thinking_levels: ["off", "low", "medium", "high"],
		});
		expect(levels).toEqual(["off", "low", "medium", "high"]);
	});

	it("treats bare supports_thinking as off + on(max)", async () => {
		const levels = await supportedLevels({ supports_thinking: true });
		expect(levels).toEqual(["off", "high"]);
	});

	it("includes xhigh when the relay advertises it", async () => {
		const levels = await supportedLevels({
			supports_thinking: true,
			thinking_levels: ["off", "high", "xhigh"],
		});
		expect(levels).toEqual(["off", "high", "xhigh"]);
	});

	it("reports only off for non-thinking models", async () => {
		const levels = await supportedLevels({ supports_thinking: false });
		expect(levels).toEqual(["off"]);
	});
});

describe("Hamr startup config relay compatibility", () => {
	const originalFetch = globalThis.fetch;
	const originalSkip = process.env.HAMR_TEST_SKIP_NETWORK;

	beforeEach(() => {
		process.env.HAMR_TEST_SKIP_NETWORK = "1";
	});

	afterEach(() => {
		globalThis.fetch = originalFetch;
		if (originalSkip === undefined) delete process.env.HAMR_TEST_SKIP_NETWORK;
		else process.env.HAMR_TEST_SKIP_NETWORK = originalSkip;
	});

	it("uses conservative OpenAI-compatible defaults for discovered-style relay models", async () => {
		const config: HamrStartupConfig = {
			active: { provider: "relay", model: "devstral-2-24b", thinking: "auto" },
			providers: {
				relay: {
					name: "Relay",
					base_url: "https://ai.watchyourtemper.com/v1",
					compatibility: "openai-compatible",
					headers: {
						"CF-Access-Client-Id": "client-id",
						"CF-Access-Client-Secret": "client-secret",
					},
					models: [
						{
							id: "devstral-2-24b",
							display_name: "Devstral 2 24B",
							context_window: 32768,
							max_output_tokens: 16384,
							supports_thinking: true,
						},
					],
				},
			},
		};

		const registrations = await buildHamrProviderRegistrations(config);
		const relay = registrations.find((registration) => registration.name === "relay");
		expect(relay).toBeDefined();
		expect(relay?.config.models?.[0]?.compat).toMatchObject({
			supportsDeveloperRole: false,
			// Opt in to streaming usage so real token counts drive the context-window %.
			supportsUsageInStreaming: true,
			supportsStrictMode: false,
			maxTokensField: "max_tokens",
		});
	});

	it("defaults configured relay models to text and image input", async () => {
		const config: HamrStartupConfig = {
			providers: {
				relay: {
					enabled: true,
					name: "Relay",
					compatibility: "openai-compatible",
					base_url: "http://127.0.0.1:1234/v1",
					models: [{ id: "devstral-2-24b" }],
				},
			},
			sourcePaths: [],
		};

		const registrations = await buildHamrProviderRegistrations(config);

		expect(registrations).toHaveLength(1);
		expect(registrations[0]?.config.models?.[0]?.input).toEqual(["text", "image"]);
	});

	it("preserves explicit text-only relay model config", async () => {
		const config: HamrStartupConfig = {
			providers: {
				relay: {
					enabled: true,
					name: "Relay",
					compatibility: "openai-compatible",
					base_url: "http://127.0.0.1:1234/v1",
					models: [{ id: "text-only", supports_vision: false }],
				},
			},
			sourcePaths: [],
		};

		const registrations = await buildHamrProviderRegistrations(config);

		expect(registrations[0]?.config.models?.[0]?.input).toEqual(["text"]);
	});

	it("does not invent a context window when relay discovery is unavailable", async () => {
		const config: HamrStartupConfig = {
			providers: {
				relay: {
					enabled: true,
					name: "Relay",
					compatibility: "openai-compatible",
					base_url: "http://127.0.0.1:1234/v1",
					models: [{ id: "unknown-context" }],
				},
			},
			sourcePaths: [],
		};

		const registrations = await buildHamrProviderRegistrations(config);

		expect(registrations[0]?.config.models?.[0]?.contextWindow).toBe(0);
		expect(registrations[0]?.config.models?.[0]?.maxTokens).toBe(16384);
	});
});

describe("Hamr relay configured model discovery merge", () => {
	const originalFetch = globalThis.fetch;
	const originalSkip = process.env.HAMR_TEST_SKIP_NETWORK;

	function mockModelsResponse(models: Array<Record<string, unknown>>): void {
		globalThis.fetch = (async () =>
			new Response(JSON.stringify({ object: "list", data: models }), { status: 200 })) as typeof fetch;
	}

	beforeEach(() => {
		delete process.env.HAMR_TEST_SKIP_NETWORK;
	});

	afterEach(() => {
		globalThis.fetch = originalFetch;
		if (originalSkip === undefined) delete process.env.HAMR_TEST_SKIP_NETWORK;
		else process.env.HAMR_TEST_SKIP_NETWORK = originalSkip;
	});

	it("hydrates configured relay models from endpoint-advertised context", async () => {
		mockModelsResponse([{ id: "qwen3.6-27b", meta: { n_ctx: 32768 }, capabilities: ["completion"] }]);
		const config: HamrStartupConfig = {
			providers: {
				relay: {
					enabled: true,
					name: "Relay",
					compatibility: "openai-compatible",
					base_url: "http://127.0.0.1:9/v1",
					models: [{ id: "qwen3.6-27b", display_name: "Qwen 3.6 27B", supports_thinking: false }],
				},
			},
			sourcePaths: [],
		};

		const registrations = await buildHamrProviderRegistrations(config);
		const model = registrations[0]?.config.models?.[0];

		expect(model?.contextWindow).toBe(32768);
		expect(model?.maxTokens).toBe(16384);
		expect(model?.name).toBe("Qwen 3.6 27B");
		expect(model?.input).toEqual(["text", "image"]);
	});

	it("treats relay-advertised context as authoritative over stale config", async () => {
		mockModelsResponse([{ id: "qwen3.6-35b-a3b", meta: { n_ctx: 98304 }, capabilities: ["completion"] }]);
		const config: HamrStartupConfig = {
			providers: {
				relay: {
					enabled: true,
					name: "Relay",
					compatibility: "openai-compatible",
					base_url: "http://127.0.0.1:9/v1",
					models: [{ id: "qwen3.6-35b-a3b", context_window: 131072, supports_thinking: true }],
				},
			},
			sourcePaths: [],
		};

		const registrations = await buildHamrProviderRegistrations(config);

		expect(registrations[0]?.config.models?.[0]?.contextWindow).toBe(98304);
	});
});

describe("Hamr cloud-provider gating", () => {
	const config: HamrStartupConfig = {
		providers: {
			relay: { name: "Relay", base_url: "http://127.0.0.1:1234/v1" },
			"local-lm": { name: "LM Studio", cloud: false, base_url: "http://127.0.0.1:1234/v1" },
			"my-cloud": { name: "My Cloud", cloud: true, base_url: "https://api.example.com/v1" },
		},
		sourcePaths: [],
	};

	it("defaults a configured relay/local provider to non-cloud", () => {
		expect(isCloudProvider(config, "relay")).toBe(false);
		expect(isCloudProvider(config, "local-lm")).toBe(false);
	});

	it("honors an explicit cloud:true flag", () => {
		expect(isCloudProvider(config, "my-cloud")).toBe(true);
	});

	it("treats unconfigured (built-in cloud) providers as cloud", () => {
		expect(isCloudProvider(config, "anthropic")).toBe(true);
		expect(isCloudProvider(config, "openai")).toBe(true);
	});
});

describe("Hamr relay discovered-model vision defaults", () => {
	const originalFetch = globalThis.fetch;
	const originalSkip = process.env.HAMR_TEST_SKIP_NETWORK;

	function mockModelsResponse(models: Array<Record<string, unknown>>): void {
		globalThis.fetch = (async () =>
			new Response(JSON.stringify({ object: "list", data: models }), { status: 200 })) as typeof fetch;
	}

	beforeEach(() => {
		delete process.env.HAMR_TEST_SKIP_NETWORK;
	});

	afterEach(() => {
		globalThis.fetch = originalFetch;
		if (originalSkip === undefined) delete process.env.HAMR_TEST_SKIP_NETWORK;
		else process.env.HAMR_TEST_SKIP_NETWORK = originalSkip;
	});

	async function discoveredInputFor(model: Record<string, unknown>): Promise<("text" | "image")[]> {
		mockModelsResponse([model]);
		const config: HamrStartupConfig = {
			providers: {
				relay: { enabled: true, name: "Relay", compatibility: "openai-compatible", base_url: "http://127.0.0.1:9/v1" },
			},
			sourcePaths: [],
		};
		const registrations = await buildHamrProviderRegistrations(config);
		return registrations[0]?.config.models?.[0]?.input as ("text" | "image")[];
	}

	it("treats a discovered model without an explicit supportsVision flag as vision-capable", async () => {
		const input = await discoveredInputFor({ id: "qwen3.6-35b-a3b", capabilities: ["completion"] });
		expect(input).toEqual(["text", "image"]);
	});

	it("enables image input when the relay advertises multimodal", async () => {
		const input = await discoveredInputFor({ id: "gemma-4-26b", capabilities: ["completion", "multimodal"] });
		expect(input).toEqual(["text", "image"]);
	});
});

describe("Hamr relay model discovery", () => {
	it("detects vision support from OpenAI-compatible model metadata", async () => {
		const originalFetch = globalThis.fetch;
		globalThis.fetch = (async () =>
			new Response(
				JSON.stringify({
					data: [
						{
							id: "local-vision-model",
							capabilities: ["completion", "vision"],
						},
					],
				}),
				{ status: 200 },
			)) as typeof fetch;

		try {
			const models = await discoverRelayModels("http://127.0.0.1:1234/v1");

			expect(models).toHaveLength(1);
			expect(models[0]?.supportsVision).toBe(true);
		} finally {
			globalThis.fetch = originalFetch;
		}
	});
});
