import { describe, expect, test } from "vitest";
import { UserMessageComponent } from "../src/modes/interactive/components/user-message.ts";
import { initTheme } from "../src/modes/interactive/theme/theme.ts";

const OSC133_ZONE_START = "\x1b]133;A\x07";
const OSC133_ZONE_END = "\x1b]133;B\x07";
const OSC133_ZONE_FINAL = "\x1b]133;C\x07";

describe("UserMessageComponent", () => {
	test("keeps user message height stable while moving closing OSC markers off line end", () => {
		initTheme("dark");

		const component = new UserMessageComponent("hello");
		const lines = component.render(20);

		expect(lines).toHaveLength(3);
		expect(lines[0]).toContain(OSC133_ZONE_START);
		expect(lines[0]).not.toContain(OSC133_ZONE_END);
		expect(lines[1]).toContain("hello");
		expect(lines[2].startsWith(OSC133_ZONE_END + OSC133_ZONE_FINAL)).toBe(true);
	});

	test("uses the neutral prompt surface instead of tinting the whole card with model color", () => {
		initTheme("dark");

		const component = new UserMessageComponent("hello", undefined, "#f06030", "M");
		const rendered = component.render(40).join("\n");

		expect(rendered).toContain("PROMPT");
		expect(rendered).toContain("hello");
		expect(rendered).not.toContain("\x1b[48;2;29;12;6m");
		// shadedSurfaces is disabled by default; no full-width background surface
		expect(rendered).not.toContain("\x1b[48;2;52;53;65m");
	});
});
