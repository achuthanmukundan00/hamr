import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CONFIG_DIR_NAME, getAgentDir } from "../src/config.ts";
import { normalizeChangelogLinks } from "../src/utils/changelog.ts";
import { getLatestHamrVersion } from "../src/utils/version-check.ts";

const repoRoot = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..", "..");

afterEach(() => {
	vi.unstubAllGlobals();
});

// ---------------------------------------------------------------------------
// Update infrastructure: Hamr must not check inherited Pi update channels.
// ---------------------------------------------------------------------------
describe("update channels are Hamr-owned", () => {
	it("queries the @skaft/hamr npm registry entry first, never a pi/earendil host", async () => {
		const fetchMock = vi.fn(async () => Response.json({ name: "@skaft/hamr", version: "9.9.9" }));
		vi.stubGlobal("fetch", fetchMock);

		await getLatestHamrVersion("1.0.0");

		expect(fetchMock).toHaveBeenCalledTimes(1);
		const url = String(fetchMock.mock.calls[0]?.[0]);
		expect(url).toBe("https://registry.npmjs.org/@skaft%2fhamr/latest");
		expect(url).not.toMatch(/earendil|pi-mono|\bpi\b/i);
	});

	it("rewrites changelog links to github.com/skaft-software/hamr, not earendil-works/pi", () => {
		const legacy = normalizeChangelogLinks(
			"[notes](https://github.com/earendil-works/pi-mono/blob/main/CHANGELOG.md)",
			"1.2.3",
		);
		expect(legacy).toContain("github.com/skaft-software/hamr");
		expect(legacy).not.toContain("earendil-works/pi");

		const relative = normalizeChangelogLinks("[readme](README.md)", "1.2.3");
		expect(relative).toContain("github.com/skaft-software/hamr");
	});
});

// ---------------------------------------------------------------------------
// Runtime config/state paths: Hamr must live under its own config dir.
// ---------------------------------------------------------------------------
describe("config paths are Hamr-owned", () => {
	it("uses the .hamr config directory", () => {
		expect(CONFIG_DIR_NAME).toBe(".hamr");
		expect(getAgentDir()).toContain(`${CONFIG_DIR_NAME}/agent`);
		expect(getAgentDir()).not.toContain("/.pi/");
	});
});

// ---------------------------------------------------------------------------
// Package metadata: every published package must be Hamr/Skaft-owned.
// ---------------------------------------------------------------------------
describe("package metadata is Hamr/Skaft-owned", () => {
	const packages = ["agent", "ai", "coding-agent", "tui"];

	for (const pkgDir of packages) {
		it(`@hamr/${pkgDir} metadata points at skaft-software/hamr`, () => {
			const pkg = JSON.parse(readFileSync(join(repoRoot, "packages", pkgDir, "package.json"), "utf-8"));
			expect(pkg.name).toMatch(/^@hamr\//);
			expect(pkg.repository?.url).toContain("skaft-software/hamr");
			expect(pkg.repository?.url).not.toContain("earendil-works/pi");
			// Original Pi authorship is preserved as attribution, but the owner is Skaft.
			expect(pkg.author).toBe("Skaft");
		});
	}

	it("coding-agent no longer exposes a piConfig manifest key", () => {
		const pkg = JSON.parse(readFileSync(join(repoRoot, "packages", "coding-agent", "package.json"), "utf-8"));
		expect(pkg.hamrConfig?.configDir).toBe(".hamr");
		expect(pkg.piConfig).toBeUndefined();
	});
});
