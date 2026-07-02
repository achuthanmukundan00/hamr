/**
 * Integration test for the HAMR_CHILD_CONFIG fast path in createAgentSession.
 *
 * A subagent child must clone the parent's model from the config snapshot:
 * - with no CLI-resolved model, the session model comes from the snapshot,
 *   preserving the parent's actual fields (baseUrl, provider) rather than a
 *   built-in registry lookalike of the same id;
 * - an explicit options.model (a per-task override) still wins.
 */
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { createAgentSession } from "../src/core/sdk.ts";

function writeChildConfig(overrides: Record<string, unknown> = {}): string {
	const configPath = path.join(os.tmpdir(), `hamr-child-config-test-${Date.now()}-${Math.random()}.json`);
	const config = {
		provider: "openai-codex",
		modelId: "gpt-5.5",
		modelName: "GPT-5.5 (parent)",
		modelApi: "openai-codex-responses",
		modelBaseUrl: "https://parent.example.com/v1",
		modelReasoning: true,
		modelContextWindow: 400000,
		modelMaxTokens: 128000,
		apiKey: "parent-key",
		toolNames: ["read"],
		systemPrompt: "test prompt",
		cwd: process.cwd(),
		treeBudgetRemaining: 10,
		...overrides,
	};
	fs.writeFileSync(configPath, JSON.stringify(config), "utf-8");
	return configPath;
}

describe("createAgentSession with HAMR_CHILD_CONFIG", () => {
	let configPath: string | undefined;

	afterEach(() => {
		delete process.env.HAMR_CHILD_CONFIG;
		if (configPath) {
			try {
				fs.unlinkSync(configPath);
			} catch {
				/* already gone */
			}
			configPath = undefined;
		}
	});

	it("clones the parent model from the snapshot when no CLI model is given", async () => {
		configPath = writeChildConfig();
		process.env.HAMR_CHILD_CONFIG = configPath;

		const { session } = await createAgentSession({});
		expect(session.model?.provider).toBe("openai-codex");
		expect(session.model?.id).toBe("gpt-5.5");
		// Snapshot fields, not the built-in registry entry of the same id.
		expect(session.model?.baseUrl).toBe("https://parent.example.com/v1");
		expect(session.model?.name).toBe("GPT-5.5 (parent)");
	});

	it("lets an explicit options.model (per-task override) win over the snapshot", async () => {
		configPath = writeChildConfig();
		process.env.HAMR_CHILD_CONFIG = configPath;

		const override = {
			provider: "deepseek",
			id: "deepseek-v4-flash",
			name: "DeepSeek V4 Flash",
			api: "openai-completions",
			baseUrl: "https://api.deepseek.com/v1",
			reasoning: false,
			thinkingLevelMap: {},
			input: ["text"],
			cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
			contextWindow: 128000,
			maxTokens: 8192,
		};
		const { session } = await createAgentSession({ model: override as never });
		expect(session.model?.provider).toBe("deepseek");
		expect(session.model?.id).toBe("deepseek-v4-flash");
	});

	it("falls back to the built-in registry when the snapshot lacks model fields", async () => {
		configPath = writeChildConfig({ modelBaseUrl: undefined, modelName: undefined, modelApi: undefined });
		process.env.HAMR_CHILD_CONFIG = configPath;

		const { session } = await createAgentSession({});
		expect(session.model?.provider).toBe("openai-codex");
		expect(session.model?.id).toBe("gpt-5.5");
		// Built-in entry supplies the fields the snapshot lacked.
		expect(session.model?.name).toBeTruthy();
	});
});
