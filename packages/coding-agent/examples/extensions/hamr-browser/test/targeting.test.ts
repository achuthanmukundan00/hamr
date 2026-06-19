import { describe, expect, it } from "vitest";
import { normalizeScrollAmount, parseTarget } from "../src/targeting.ts";

describe("target parsing", () => {
  it("keeps explicit css selectors", () => {
    expect(parseTarget("css=#submit")).toEqual({ kind: "css", value: "#submit" });
  });

  it("recognizes role targets", () => {
    expect(parseTarget("role=button:Submit")).toEqual({ kind: "role", role: "button", name: "Submit" });
  });

  it("uses text matching for plain strings", () => {
    expect(parseTarget("Continue")).toEqual({ kind: "text", value: "Continue" });
  });
});

describe("scroll amount parsing", () => {
  it("maps directions to default pixel amounts", () => {
    expect(normalizeScrollAmount("down")).toEqual({ x: 0, y: 600 });
    expect(normalizeScrollAmount("up")).toEqual({ x: 0, y: -600 });
  });

  it("accepts numeric pixel strings", () => {
    expect(normalizeScrollAmount("-250")).toEqual({ x: 0, y: -250 });
  });
});
