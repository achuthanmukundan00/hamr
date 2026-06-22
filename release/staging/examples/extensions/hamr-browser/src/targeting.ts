import type { Locator, Page } from "playwright";

export type ParsedTarget =
	| { kind: "css"; value: string }
	| { kind: "text"; value: string }
	| { kind: "role"; role: string; name: string };

const DEFAULT_SCROLL_PIXELS = 600;

function looksLikeCssSelector(target: string): boolean {
	return /^[#.\[]/.test(target) || /^[a-z][\w-]*(?:[#.\[:\s>+~]|$)/.test(target);
}

export function parseTarget(input: string): ParsedTarget {
	const target = input.trim();
	if (!target) {
		throw new Error("Target is required");
	}

	if (target.startsWith("css=")) {
		return { kind: "css", value: target.slice("css=".length).trim() };
	}
	if (target.startsWith("text=")) {
		return { kind: "text", value: target.slice("text=".length).trim() };
	}
	if (target.startsWith("role=")) {
		const roleTarget = target.slice("role=".length);
		const separator = roleTarget.indexOf(":");
		if (separator === -1) {
			throw new Error("Role target must look like role=button:Submit");
		}
		return {
			kind: "role",
			role: roleTarget.slice(0, separator).trim(),
			name: roleTarget.slice(separator + 1).trim(),
		};
	}
	if (looksLikeCssSelector(target)) {
		return { kind: "css", value: target };
	}
	return { kind: "text", value: target };
}

export function locatorForTarget(page: Page, target: string): Locator {
	const parsed = parseTarget(target);
	switch (parsed.kind) {
		case "css":
			return page.locator(parsed.value).first();
		case "role":
			return page.getByRole(parsed.role as never, { name: parsed.name }).first();
		case "text":
			return page.getByText(parsed.value, { exact: false }).first();
	}
}

export function normalizeScrollAmount(directionOrPixels: string | number): { x: number; y: number } {
	if (typeof directionOrPixels === "number") {
		return { x: 0, y: directionOrPixels };
	}

	const value = directionOrPixels.trim().toLowerCase();
	const numeric = Number(value);
	if (Number.isFinite(numeric)) {
		return { x: 0, y: numeric };
	}

	switch (value) {
		case "up":
			return { x: 0, y: -DEFAULT_SCROLL_PIXELS };
		case "down":
			return { x: 0, y: DEFAULT_SCROLL_PIXELS };
		case "left":
			return { x: -DEFAULT_SCROLL_PIXELS, y: 0 };
		case "right":
			return { x: DEFAULT_SCROLL_PIXELS, y: 0 };
		default:
			throw new Error("Scroll must be up, down, left, right, or a pixel count");
	}
}
