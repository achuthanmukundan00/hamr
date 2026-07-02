/**
 * Regression test for worker model resolution (provider ambiguity).
 *
 * Workers are spawned as `hamr --model <spec>`. When the parent model was
 * forwarded as a bare id (e.g. "gpt-5.5"), the child's CLI resolver picked
 * the FIRST provider in the registry with that exact id — for gpt-5.5 that
 * is azure-openai-responses, not the parent's openai-codex — and the worker
 * died instantly with "No API key found for azure-openai-responses".
 *
 * The parent-inherited model must therefore always be provider-qualified
 * ("provider/id"). Explicit per-task overrides are passed through verbatim.
 */
import { describe, expect, it } from "vitest";
import { _testExports } from "../src/hamr/extensions/subagents.ts";

const { resolveWorkerModelSpec, buildWorkerCliArgs } = _testExports;

describe("resolveWorkerModelSpec", () => {
	it("qualifies the inherited parent model with its provider", () => {
		expect(resolveWorkerModelSpec(undefined, { provider: "openai-codex", id: "gpt-5.5" })).toBe("openai-codex/gpt-5.5");
	});

	it("passes an explicit per-task model override through verbatim", () => {
		expect(resolveWorkerModelSpec("deepseek/deepseek-v4-flash", { provider: "openai-codex", id: "gpt-5.5" })).toBe(
			"deepseek/deepseek-v4-flash",
		);
		expect(resolveWorkerModelSpec("deepseek-v4-flash", undefined)).toBe("deepseek-v4-flash");
	});

	it("returns undefined when there is no override and no parent model", () => {
		expect(resolveWorkerModelSpec(undefined, undefined)).toBeUndefined();
	});

	it("does not double-qualify a parent id that already contains the provider prefix", () => {
		expect(resolveWorkerModelSpec(undefined, { provider: "openai-codex", id: "openai-codex/gpt-5.5" })).toBe(
			"openai-codex/gpt-5.5",
		);
	});
});

describe("buildWorkerCliArgs", () => {
	it("omits --model when the child config snapshot carries the inherited model", () => {
		const args = buildWorkerCliArgs({
			task: "do things",
			inheritedModelSpec: "openai-codex/gpt-5.5",
			hasChildConfig: true,
		});
		expect(args).not.toContain("--model");
		expect(args[args.length - 1]).toBe("do things");
	});

	it("falls back to the provider-qualified inherited model when the snapshot is missing", () => {
		const args = buildWorkerCliArgs({
			task: "do things",
			inheritedModelSpec: "openai-codex/gpt-5.5",
			hasChildConfig: false,
		});
		expect(args.join(" ")).toContain("--model openai-codex/gpt-5.5");
	});

	it("always passes an explicit per-task override, snapshot or not", () => {
		for (const hasChildConfig of [true, false]) {
			const args = buildWorkerCliArgs({
				task: "do things",
				workerModel: "deepseek/deepseek-v4-flash",
				inheritedModelSpec: "openai-codex/gpt-5.5",
				hasChildConfig,
			});
			expect(args.join(" ")).toContain("--model deepseek/deepseek-v4-flash");
			expect(args.join(" ")).not.toContain("gpt-5.5");
		}
	});

	it("passes tools and keeps the task as the final positional argument", () => {
		const args = buildWorkerCliArgs({
			task: "review the code",
			hasChildConfig: true,
			workerTools: ["read", "grep"],
		});
		expect(args.join(" ")).toContain("--tools read,grep");
		expect(args[args.length - 1]).toBe("review the code");
	});
});
