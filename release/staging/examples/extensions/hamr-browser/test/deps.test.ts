import { describe, expect, it } from "vitest";
import { getBrowserDepsDir, getBrowserDepsInstallCommand, getHamrAgentDir } from "../src/deps.ts";

describe("Hamr Browser dependency install paths", () => {
	it("uses the Hamr agent dir for lazy browser dependencies", () => {
		expect(getHamrAgentDir({ HAMR_CODING_AGENT_DIR: "/tmp/hamr-agent" })).toBe("/tmp/hamr-agent");
		expect(getBrowserDepsDir({ HAMR_CODING_AGENT_DIR: "/tmp/hamr-agent" })).toBe("/tmp/hamr-agent/browser-deps");
	});

	it("allows explicitly overriding the dependency directory", () => {
		expect(getBrowserDepsDir({ HAMR_BROWSER_DEPS_DIR: "/tmp/browser-deps" })).toBe("/tmp/browser-deps");
	});

	it("installs Playwright into the lazy dependency directory", () => {
		expect(getBrowserDepsInstallCommand("/tmp/browser-deps")).toEqual([
			"npm",
			"install",
			"--prefix",
			"/tmp/browser-deps",
			"playwright@^1.57.0",
		]);
	});
});
