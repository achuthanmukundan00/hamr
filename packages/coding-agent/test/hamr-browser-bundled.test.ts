import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createAgentSessionServices } from "../src/core/agent-session-services.ts";

const packageRoot = join(import.meta.dirname, "..");

describe("bundled Hamr Browser package", () => {
	let tempDir: string;

	beforeEach(() => {
		tempDir = join(tmpdir(), `hamr-browser-bundled-${Date.now()}-${Math.random().toString(36).slice(2)}`);
		mkdirSync(tempDir, { recursive: true });
	});

	afterEach(() => {
		rmSync(tempDir, { recursive: true, force: true });
	});

	it("is included in the standard package manifest", () => {
		const pkg = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8"));
		expect(pkg.hamr?.extensions).toContain("./examples/extensions/hamr-browser/index.ts");
		expect(pkg.hamr?.skills).toContain("./examples/extensions/hamr-browser/skills");
		expect(pkg.pi?.extensions).toContain("./examples/extensions/hamr-browser/index.ts");
		expect(pkg.pi?.skills).toContain("./examples/extensions/hamr-browser/skills");
	});

	it("ships the extension entry point and browser skill", () => {
		expect(existsSync(join(packageRoot, "examples/extensions/hamr-browser/index.ts"))).toBe(true);
		expect(existsSync(join(packageRoot, "examples/extensions/hamr-browser/skills/hamr-browser.md"))).toBe(true);
	});

	it("loads browser tools and skill in standard services without user configuration", async () => {
		const cwd = join(tempDir, "project");
		const agentDir = join(tempDir, "agent");
		mkdirSync(cwd, { recursive: true });
		mkdirSync(agentDir, { recursive: true });

		const services = await createAgentSessionServices({
			cwd,
			agentDir,
			resourceLoaderOptions: { noContextFiles: true },
		});

		const extension = services.resourceLoader
			.getExtensions()
			.extensions.find((candidate) => candidate.path.includes("hamr-browser"));
		expect(extension?.tools.has("browser_launch")).toBe(true);
		expect(services.resourceLoader.getSkills().skills.some((skill) => skill.name === "hamr-browser")).toBe(true);
	});
});
