import { describe, expect, it } from "vitest";
import { getHamrUserAgent } from "../src/utils/hamr-user-agent.ts";

describe("getHamrUserAgent", () => {
	it("formats the user agent expected by hamr.dev", () => {
		const runtime = process.versions.bun ? `bun/${process.versions.bun}` : `node/${process.version}`;
		const userAgent = getHamrUserAgent("1.2.3");

		expect(userAgent).toBe(`hamr/1.2.3 (${process.platform}; ${runtime}; ${process.arch})`);
		expect(userAgent).toMatch(/^hamr\/[^\s()]+ \([^;()]+;\s*[^;()]+;\s*[^()]+\)$/);
	});
});
