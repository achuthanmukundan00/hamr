import { describe, expect, it } from "vitest";
import { getDefaultBrowserProfileDir, getDefaultScreenshotDir } from "../src/paths.ts";

describe("Hamr Browser paths", () => {
  it("uses ~/.hamr/browser-profile on macOS and Linux", () => {
    expect(getDefaultBrowserProfileDir("darwin", "/Users/alice", {})).toBe("/Users/alice/.hamr/browser-profile");
    expect(getDefaultBrowserProfileDir("linux", "/home/alice", {})).toBe("/home/alice/.hamr/browser-profile");
  });

  it("uses LOCALAPPDATA on Windows when available", () => {
    expect(
      getDefaultBrowserProfileDir("win32", "C:/Users/Alice", { LOCALAPPDATA: "C:/Users/Alice/AppData/Local" }),
    ).toBe("C:/Users/Alice/AppData/Local/Hamr/browser-profile");
  });

  it("falls back to home AppData/Local on Windows", () => {
    expect(getDefaultBrowserProfileDir("win32", "C:/Users/Alice", {})).toBe(
      "C:/Users/Alice/AppData/Local/Hamr/browser-profile",
    );
  });

  it("stores screenshots under a Hamr browser artifacts directory", () => {
    expect(getDefaultScreenshotDir("/home/alice")).toBe("/home/alice/.hamr/browser-artifacts/screenshots");
  });
});
