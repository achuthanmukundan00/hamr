import { afterEach, describe, expect, it, vi } from "vitest";
import {
	checkForNewPiVersion,
	comparePackageVersions,
	getLatestHamrRelease,
	getLatestPiRelease,
	getLatestPiVersion,
	isNewerPackageVersion,
} from "../src/utils/version-check.ts";

const originalHamrSkipVersionCheck = process.env.HAMR_SKIP_VERSION_CHECK;
const originalSkipVersionCheck = process.env.PI_SKIP_VERSION_CHECK;
const originalHamrOffline = process.env.HAMR_OFFLINE;
const originalOffline = process.env.PI_OFFLINE;

afterEach(() => {
	vi.unstubAllGlobals();
	if (originalHamrSkipVersionCheck === undefined) {
		delete process.env.HAMR_SKIP_VERSION_CHECK;
	} else {
		process.env.HAMR_SKIP_VERSION_CHECK = originalHamrSkipVersionCheck;
	}
	if (originalSkipVersionCheck === undefined) {
		delete process.env.PI_SKIP_VERSION_CHECK;
	} else {
		process.env.PI_SKIP_VERSION_CHECK = originalSkipVersionCheck;
	}
	if (originalHamrOffline === undefined) {
		delete process.env.HAMR_OFFLINE;
	} else {
		process.env.HAMR_OFFLINE = originalHamrOffline;
	}
	if (originalOffline === undefined) {
		delete process.env.PI_OFFLINE;
	} else {
		process.env.PI_OFFLINE = originalOffline;
	}
});

describe("version checks", () => {
	it("compares package versions", () => {
		expect(comparePackageVersions("0.70.6", "0.70.5")).toBeGreaterThan(0);
		expect(comparePackageVersions("0.70.5", "0.70.5")).toBe(0);
		expect(comparePackageVersions("0.70.4", "0.70.5")).toBeLessThan(0);
		expect(comparePackageVersions("5.0.0-beta.20", "5.0.0-beta.9")).toBeGreaterThan(0);
		expect(isNewerPackageVersion("0.70.5", "0.70.5")).toBe(false);
		expect(isNewerPackageVersion("0.70.6", "0.70.5")).toBe(true);
	});

	it("returns only newer versions", async () => {
		const fetchMock = vi.fn(async () => Response.json({ name: "@skaft/hamr", version: "1.2.3" }));
		vi.stubGlobal("fetch", fetchMock);

		await expect(checkForNewPiVersion("1.2.3")).resolves.toBeUndefined();
		await expect(checkForNewPiVersion("1.2.2")).resolves.toEqual({
			packageName: "@skaft/hamr",
			version: "1.2.3",
		});
	});

	it("uses the @skaft/hamr npm registry entry as the primary version check with a hamr user agent", async () => {
		const fetchMock = vi.fn(async () => Response.json({ name: "@skaft/hamr", version: "1.2.4" }));
		vi.stubGlobal("fetch", fetchMock);

		await expect(getLatestPiVersion("1.2.3")).resolves.toBe("1.2.4");
		expect(fetchMock).toHaveBeenCalledWith(
			"https://registry.npmjs.org/@skaft%2fhamr/latest",
			expect.objectContaining({
				headers: expect.objectContaining({
					"User-Agent": expect.stringMatching(/^hamr\/1\.2\.3 /),
					accept: "application/json",
				}),
			}),
		);
	});

	it("normalizes npm package metadata to the published @skaft/hamr package", async () => {
		const fetchMock = vi.fn(async () => Response.json({ name: "@some/mirror", version: "1.2.4" }));
		vi.stubGlobal("fetch", fetchMock);

		await expect(getLatestPiRelease("1.2.3")).resolves.toEqual({
			packageName: "@skaft/hamr",
			version: "1.2.4",
		});
	});

	it("returns update notes when npm metadata includes them", async () => {
		const fetchMock = vi.fn(async () => Response.json({ note: " **Read this** ", version: "1.2.4" }));
		vi.stubGlobal("fetch", fetchMock);

		await expect(getLatestPiRelease("1.2.3")).resolves.toEqual({
			note: "**Read this**",
			packageName: "@skaft/hamr",
			version: "1.2.4",
		});
	});

	it("falls back to the hamr.dev metadata endpoint if npm is unavailable", async () => {
		const fetchMock = vi
			.fn()
			.mockRejectedValueOnce(new Error("npm unavailable"))
			.mockResolvedValueOnce(Response.json({ note: " fallback note ", packageName: "@skaft/hamr", version: "1.2.4" }));
		vi.stubGlobal("fetch", fetchMock);

		await expect(getLatestHamrRelease("1.2.3")).resolves.toEqual({
			note: "fallback note",
			packageName: "@skaft/hamr",
			version: "1.2.4",
		});
		expect(fetchMock).toHaveBeenNthCalledWith(1, "https://registry.npmjs.org/@skaft%2fhamr/latest", expect.any(Object));
		expect(fetchMock).toHaveBeenNthCalledWith(2, "https://hamr.dev/api/latest-version", expect.any(Object));
	});

	it("skips api calls when version checks are disabled", async () => {
		process.env.PI_SKIP_VERSION_CHECK = "1";
		const fetchMock = vi.fn();
		vi.stubGlobal("fetch", fetchMock);

		await expect(getLatestPiVersion("1.2.3")).resolves.toBeUndefined();
		expect(fetchMock).not.toHaveBeenCalled();
	});
});
