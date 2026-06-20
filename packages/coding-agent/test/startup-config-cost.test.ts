import { describe, expect, it } from "vitest";
import { buildHamrProviderRegistrations, type HamrStartupConfig } from "../src/hamr/startup-config.ts";

/**
 * Regression: a `.hamr.toml` entry that shadows a built-in provider+model must
 * inherit the built-in's pricing (pi's modelOverride semantics), rather than
 * zeroing it. Without this, footer/ledger cost stays $0.000 for paid models
 * configured via TOML. Anthropic is used because its (anthropic-compatible)
 * path resolves models without network discovery.
 */
describe("buildHamrProviderRegistrations cost inheritance", () => {
	const baseConfig = (model: Record<string, unknown>): HamrStartupConfig => ({
		providers: {
			anthropic: {
				enabled: true,
				name: "Anthropic",
				compatibility: "anthropic-compatible",
				base_url: "https://api.anthropic.com/v1",
				api_key: "test-key",
				models: [model],
			} as HamrStartupConfig["providers"][string],
		},
		sourcePaths: [],
	});

	it("inherits built-in cost when the TOML model omits it", async () => {
		const regs = await buildHamrProviderRegistrations(
			baseConfig({ id: "claude-haiku-4-5", display_name: "Claude Haiku 4.5" }),
		);
		const model = regs[0]?.config.models?.[0];
		expect(model?.cost).toEqual({ input: 1, output: 5, cacheRead: 0.1, cacheWrite: 1.25 });
	});

	it("lets an explicit TOML cost override the built-in", async () => {
		const regs = await buildHamrProviderRegistrations(
			baseConfig({
				id: "claude-haiku-4-5",
				cost: { input: 9, output: 9, cacheRead: 9, cacheWrite: 9 },
			}),
		);
		expect(regs[0]?.config.models?.[0]?.cost).toEqual({ input: 9, output: 9, cacheRead: 9, cacheWrite: 9 });
	});

	it("falls back to zero cost for an unknown (non-built-in) model id", async () => {
		const regs = await buildHamrProviderRegistrations(baseConfig({ id: "not-a-real-model" }));
		expect(regs[0]?.config.models?.[0]?.cost).toEqual({ input: 0, output: 0, cacheRead: 0, cacheWrite: 0 });
	});
});
