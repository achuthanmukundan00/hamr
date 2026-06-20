import assert from "node:assert";
import { describe, it } from "node:test";
import { SettingsList } from "../src/components/settings-list.ts";

const testTheme = {
	label: (text: string) => text,
	value: (text: string) => text,
	description: (text: string) => text,
	section: (text: string) => text,
	cursor: "> ",
	hint: (text: string) => text,
};

describe("SettingsList", () => {
	it("sets wantsKeyRelease so Kitty protocol release events reach handleInput", () => {
		const list = new SettingsList(
			[],
			10,
			testTheme,
			() => {},
			() => {},
		);
		assert.strictEqual(list.wantsKeyRelease, true);
	});

	it("calls onCancel when escape is pressed", () => {
		let cancelled = false;
		const list = new SettingsList(
			[{ id: "a", label: "A", currentValue: "on" }],
			10,
			testTheme,
			() => {},
			() => {
				cancelled = true;
			},
		);
		list.handleInput("\x1b");
		assert.strictEqual(cancelled, true);
	});

	it("calls onCancel when Kitty escape press is received", () => {
		let cancelled = false;
		const list = new SettingsList(
			[{ id: "a", label: "A", currentValue: "on" }],
			10,
			testTheme,
			() => {},
			() => {
				cancelled = true;
			},
		);
		// Kitty escape press: \x1b[27;1:1u
		list.handleInput("\x1b[27;1:1u");
		assert.strictEqual(cancelled, true);
	});

	it("calls onCancel when Kitty escape release is received (wantsKeyRelease)", () => {
		let cancelled = false;
		const list = new SettingsList(
			[{ id: "a", label: "A", currentValue: "on" }],
			10,
			testTheme,
			() => {},
			() => {
				cancelled = true;
			},
		);
		// Kitty escape release: \x1b[27;1:3u
		list.handleInput("\x1b[27;1:3u");
		assert.strictEqual(cancelled, true);
	});

	it("calls onCancel when ctrl+c is pressed", () => {
		let cancelled = false;
		const list = new SettingsList(
			[{ id: "a", label: "A", currentValue: "on" }],
			10,
			testTheme,
			() => {},
			() => {
				cancelled = true;
			},
		);
		list.handleInput("\x03");
		assert.strictEqual(cancelled, true);
	});
});
