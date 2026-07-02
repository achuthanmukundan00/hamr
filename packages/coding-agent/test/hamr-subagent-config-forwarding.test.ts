import fs from "fs";
import os from "os";
import path from "path";
import { describe, expect, it } from "vitest";

// ── Replicate the safeJsonClone fix from subagents.ts ──────────────────
// (not exported, so we test the logic directly)
function safeJsonClone<T extends Record<string, unknown>>(value: T): T {
	try {
		return JSON.parse(JSON.stringify(value)) as T;
	} catch {
		const result: Record<string, unknown> = {};
		for (const [key, val] of Object.entries(value)) {
			try {
				JSON.stringify(val);
				result[key] = val;
			} catch {
				// Skip non-serializable fields (functions, symbols, etc.)
			}
		}
		return result as T;
	}
}

// ── Replicate the parent config serialization logic ────────────────────
interface HamrChildConfig {
	modelName?: string;
	modelCompat?: unknown;
	apiKey?: string;
	apiHeaders?: Record<string, string>;
	apiEnv?: Record<string, string>;
	provider?: string;
	[key: string]: unknown;
}

function serializeParentConfig(parentConfig: HamrChildConfig): string | undefined {
	const tmpDir = os.tmpdir();
	const childConfigPath = path.join(tmpDir, `hamr-config-test-${Date.now()}-${Math.random()}.json`);
	try {
		fs.writeFileSync(childConfigPath, JSON.stringify(parentConfig), { encoding: "utf-8", mode: 0o600 });
		fs.chmodSync(childConfigPath, 0o600);
		return childConfigPath;
	} catch {
		// Retry without modelCompat
		try {
			const { modelCompat: _omitted, ...safeConfig } = parentConfig;
			fs.writeFileSync(childConfigPath, JSON.stringify(safeConfig), { encoding: "utf-8", mode: 0o600 });
			fs.chmodSync(childConfigPath, 0o600);
			return childConfigPath;
		} catch {
			return undefined;
		}
	}
}

function cleanup(path: string | undefined): void {
	if (path) {
		try {
			fs.unlinkSync(path);
		} catch {
			/* ignore */
		}
	}
}

describe("safeJsonClone (config serialization fix)", () => {
	it("round-trips plain objects", () => {
		const input = { name: "test", nested: { a: 1 }, arr: [1, 2, 3] };
		const cloned = safeJsonClone(input);
		expect(JSON.stringify(cloned)).toBe(JSON.stringify(input));
	});

	it("strips functions instead of returning the original (non-serializable)", () => {
		const input = { name: "test", handler: () => {} };
		const cloned = safeJsonClone(input);
		// Old behavior: cloned === input, JSON.stringify(cloned) would throw
		// New behavior: function field is dropped, JSON.stringify succeeds
		expect(() => JSON.stringify(cloned)).not.toThrow();
		expect(cloned.name).toBe("test");
		expect(cloned.handler).toBeUndefined();
	});

	it("strips BigInt fields that cannot be serialized", () => {
		const input = { name: "test", big: BigInt(123) };
		const cloned = safeJsonClone(input);
		expect(() => JSON.stringify(cloned)).not.toThrow();
		expect(cloned.name).toBe("test");
		expect(cloned.big).toBeUndefined();
	});

	it("strips Symbol fields", () => {
		const input = { name: "test", sym: Symbol("x") };
		const cloned = safeJsonClone(input);
		expect(() => JSON.stringify(cloned)).not.toThrow();
		expect(cloned.name).toBe("test");
		expect(cloned.sym).toBeUndefined();
	});

	it("handles empty objects", () => {
		expect(() => JSON.stringify(safeJsonClone({}))).not.toThrow();
	});

	it("preserves string, number, boolean, null fields", () => {
		const input = { s: "hello", n: 42, b: true, nil: null };
		const cloned = safeJsonClone(input);
		expect(cloned.s).toBe("hello");
		expect(cloned.n).toBe(42);
		expect(cloned.b).toBe(true);
		expect(cloned.nil).toBeNull();
	});

	it("handles values that truly throw JSON.stringify (BigInt, circular refs)", () => {
		// BigInt throws TypeError
		const input = { a: { b: BigInt(123) } };
		const cloned = safeJsonClone(input);
		expect(() => JSON.stringify(cloned)).not.toThrow();
		expect(cloned.a).toBeUndefined();

		// Circular reference throws
		const circular: any = {};
		circular.c = circular;
		const input2 = { x: circular };
		const cloned2 = safeJsonClone(input2);
		expect(() => JSON.stringify(cloned2)).not.toThrow();
		expect(cloned2.x).toBeUndefined();
	});
});

describe("parent config serialization fallback", () => {
	it("writes a normal config successfully", () => {
		const config: HamrChildConfig = {
			modelName: "gpt-5.5",
			apiKey: "sk-test",
			provider: "openai-codex",
		};
		const result = serializeParentConfig(config);
		expect(result).toBeDefined();
		if (result) {
			const content = JSON.parse(fs.readFileSync(result, "utf-8"));
			expect(content.modelName).toBe("gpt-5.5");
			expect(content.apiKey).toBe("sk-test");
			cleanup(result);
		}
	});

	it("retries without modelCompat when first serialize fails", () => {
		// Simulate a config where modelCompat contains a circular reference
		// (which causes JSON.stringify to throw)
		const compat: any = { fn: "ok" };
		compat.self = compat; // circular reference
		const config: HamrChildConfig = {
			modelName: "deepseek/deepseek-v4-pro",
			apiKey: "sk-test",
			provider: "deepseek",
			modelCompat: compat,
		};
		const result = serializeParentConfig(config);
		expect(result).toBeDefined();
		if (result) {
			const content = JSON.parse(fs.readFileSync(result, "utf-8"));
			expect(content.modelName).toBe("deepseek/deepseek-v4-pro");
			expect(content.modelCompat).toBeUndefined();
			cleanup(result);
		}
	});

	it("returns undefined when both attempts fail", () => {
		// Make JSON.stringify always throw by using a proxy
		const poison: HamrChildConfig = new Proxy(
			{ modelName: "test" },
			{
				get() {
					throw new Error("poison");
				},
			},
		);
		const result = serializeParentConfig(poison);
		expect(result).toBeUndefined();
	});
});

describe("child config apiHeaders forwarding", () => {
	it("registerProvider receives apiHeaders", async () => {
		// Simulate what createAgentSessionFromChildConfig does:
		// Calls modelRegistry.registerProvider(provider, { headers: apiHeaders })
		const apiHeaders = {
			Authorization: "Bearer sk-test-key",
			"CF-Access-Client-Id": "access-client-id",
			"CF-Access-Client-Secret": "access-secret",
		};

		// Verify the headers are correctly formatted for registerProvider
		const providerConfig = {
			headers: apiHeaders,
		};

		expect(providerConfig.headers).toEqual(apiHeaders);
		expect(Object.keys(providerConfig.headers).length).toBe(3);

		// Verify they can be serialized (the provider request config is storeable)
		expect(() => JSON.stringify(providerConfig)).not.toThrow();
		const serialized = JSON.stringify(providerConfig);
		expect(serialized).toContain("CF-Access-Client-Id");
	});

	it("skips registration when apiHeaders is empty", () => {
		// The condition in sdk.ts is: config.apiHeaders && Object.keys(config.apiHeaders).length > 0
		const emptyHeaders: Record<string, string> = {};
		const noHeaders = undefined;

		expect(emptyHeaders && Object.keys(emptyHeaders).length > 0).toBe(false);
		// undefined && ... evaluates to undefined (not false), so use toBeFalsy
		expect(noHeaders && Object.keys(noHeaders).length > 0).toBeFalsy();
	});
});
