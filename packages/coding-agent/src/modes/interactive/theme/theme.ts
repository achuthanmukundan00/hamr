import * as fs from "node:fs";
import * as path from "node:path";
import {
	type EditorTheme,
	getCapabilities,
	type MarkdownTheme,
	type RgbColor,
	type SelectListTheme,
	type SettingsListTheme,
} from "@hamr/tui";
import chalk from "chalk";
import { type Static, Type } from "typebox";
import { Compile } from "typebox/compile";
import { getCustomThemesDir, getThemesDir } from "../../../config.ts";
import type { SourceInfo } from "../../../core/source-info.ts";
import { closeWatcher, watchWithErrorHandler } from "../../../utils/fs-watch.ts";
import { highlight, supportsLanguage } from "../../../utils/syntax-highlight.ts";

// ============================================================================
// Types & Schema
// ============================================================================

const ColorValueSchema = Type.Union([
	Type.String(), // hex "#ff0000", var ref "primary", or empty ""
	Type.Integer({ minimum: 0, maximum: 255 }), // 256-color index
]);

type ColorValue = Static<typeof ColorValueSchema>;

const ThemeJsonSchema = Type.Object({
	$schema: Type.Optional(Type.String()),
	name: Type.String(),
	modelAdaptive: Type.Optional(Type.Boolean()),
	vars: Type.Optional(Type.Record(Type.String(), ColorValueSchema)),
	colors: Type.Object({
		// Core UI (10 colors)
		accent: ColorValueSchema,
		border: ColorValueSchema,
		borderAccent: ColorValueSchema,
		borderMuted: ColorValueSchema,
		success: ColorValueSchema,
		error: ColorValueSchema,
		warning: ColorValueSchema,
		muted: ColorValueSchema,
		dim: ColorValueSchema,
		text: ColorValueSchema,
		thinkingText: ColorValueSchema,
		// Backgrounds & Content Text (11 colors)
		selectedBg: ColorValueSchema,
		userMessageBg: ColorValueSchema,
		userMessageText: ColorValueSchema,
		customMessageBg: ColorValueSchema,
		customMessageText: ColorValueSchema,
		customMessageLabel: ColorValueSchema,
		toolPendingBg: ColorValueSchema,
		toolSuccessBg: ColorValueSchema,
		toolErrorBg: ColorValueSchema,
		toolTitle: ColorValueSchema,
		toolOutput: ColorValueSchema,
		// Markdown (10 colors)
		mdHeading: ColorValueSchema,
		mdLink: ColorValueSchema,
		mdLinkUrl: ColorValueSchema,
		mdCode: ColorValueSchema,
		mdCodeBlock: ColorValueSchema,
		mdCodeBlockBorder: ColorValueSchema,
		mdQuote: ColorValueSchema,
		mdQuoteBorder: ColorValueSchema,
		mdHr: ColorValueSchema,
		mdListBullet: ColorValueSchema,
		// Tool Diffs (5 colors)
		toolDiffAdded: ColorValueSchema,
		toolDiffRemoved: ColorValueSchema,
		toolDiffContext: ColorValueSchema,
		toolDiffAddedBg: ColorValueSchema,
		toolDiffRemovedBg: ColorValueSchema,
		// Syntax Highlighting (9 colors)
		syntaxComment: ColorValueSchema,
		syntaxKeyword: ColorValueSchema,
		syntaxFunction: ColorValueSchema,
		syntaxVariable: ColorValueSchema,
		syntaxString: ColorValueSchema,
		syntaxNumber: ColorValueSchema,
		syntaxType: ColorValueSchema,
		syntaxOperator: ColorValueSchema,
		syntaxPunctuation: ColorValueSchema,
		// Thinking Level Borders (6 colors)
		thinkingOff: ColorValueSchema,
		thinkingMinimal: ColorValueSchema,
		thinkingLow: ColorValueSchema,
		thinkingMedium: ColorValueSchema,
		thinkingHigh: ColorValueSchema,
		thinkingXhigh: ColorValueSchema,
		// Bash Mode (1 color)
		bashMode: ColorValueSchema,
		// Extended UI (12 colors — present for future use; pi TUI doesn't consume these yet)
		editorBg: ColorValueSchema,
		editorFg: ColorValueSchema,
		editorCursor: ColorValueSchema,
		editorSelection: ColorValueSchema,
		editorLineNumber: ColorValueSchema,
		statusBarBg: ColorValueSchema,
		surfaceBg: ColorValueSchema,
		cardBg: ColorValueSchema,
		thinkingBg: ColorValueSchema,
		toolWarningBg: ColorValueSchema,
	}),
	export: Type.Optional(
		Type.Object({
			pageBg: Type.Optional(ColorValueSchema),
			cardBg: Type.Optional(ColorValueSchema),
			infoBg: Type.Optional(ColorValueSchema),
		}),
	),
	// Legacy layout block (card padding). Superseded by `cards` but still honored.
	layout: Type.Optional(
		Type.Object({
			cardPadX: Type.Optional(Type.Number()),
			cardPadY: Type.Optional(Type.Number()),
		}),
	),
	// Message-card presentation. All optional; omitted keys fall back to
	// DEFAULT_CARD_CONFIG so themes that don't know about cards degrade to the
	// stock layout. This is what makes the "hamr look" a portable theme.
	cards: Type.Optional(
		Type.Object({
			showHeadings: Type.Optional(Type.Boolean()),
			headingGlyph: Type.Optional(Type.String()),
			promptLabel: Type.Optional(Type.String()),
			responseLabel: Type.Optional(Type.String()),
			thoughtLabel: Type.Optional(Type.String()),
			headingIndent: Type.Optional(Type.Number()),
			bodyIndent: Type.Optional(Type.Number()),
			toolIndent: Type.Optional(Type.Number()),
			cardPadX: Type.Optional(Type.Number()),
			cardPadY: Type.Optional(Type.Number()),
			thinkingShaded: Type.Optional(Type.Boolean()),
			gaplessCards: Type.Optional(Type.Boolean()),
		}),
	),
});

type ThemeJson = Static<typeof ThemeJsonSchema>;

/**
 * Resolved message-card presentation. Drives how user/assistant/tool cards are
 * laid out (labels, glyph, indents, padding, shading) so the look lives in the
 * theme (data) rather than hardcoded in the components.
 */
export interface CardConfig {
	/** Whether to render the glyph + label heading above card bodies. */
	showHeadings: boolean;
	/** "model" → active model glyph; "" → no glyph; any other string → literal glyph. */
	headingGlyph: string;
	promptLabel: string;
	responseLabel: string;
	thoughtLabel: string;
	/** Left padding of the heading label within the card. */
	headingIndent: number;
	/** Left padding of the body (markdown) within the card. */
	bodyIndent: number;
	/** Left indent applied to tool/bash output so it shares the card margin. */
	toolIndent: number;
	/** Horizontal/vertical padding of the card Box. */
	cardPadX: number;
	cardPadY: number;
	/** Whether the THOUGHT card uses the model-accent shaded background. */
	thinkingShaded: boolean;
	/** When true, no spacer is inserted between consecutive cards. */
	gaplessCards: boolean;
}

export const DEFAULT_CARD_CONFIG: CardConfig = {
	showHeadings: true,
	headingGlyph: "model",
	promptLabel: "PROMPT",
	responseLabel: "RESPONSE",
	thoughtLabel: "THOUGHT",
	headingIndent: 1,
	bodyIndent: 3,
	toolIndent: 2,
	cardPadX: 1,
	cardPadY: 1,
	thinkingShaded: false,
	gaplessCards: true,
};

/**
 * Merge a theme JSON's `cards` (and legacy `layout`) blocks over the defaults.
 * `cards.cardPadX/Y` win over the legacy `layout.cardPadX/Y`.
 */
export function resolveCardConfig(json: {
	layout?: { cardPadX?: number; cardPadY?: number };
	cards?: Partial<CardConfig>;
}): CardConfig {
	const layout = json.layout ?? {};
	const cards = json.cards ?? {};
	return {
		...DEFAULT_CARD_CONFIG,
		...cards,
		cardPadX: cards.cardPadX ?? layout.cardPadX ?? DEFAULT_CARD_CONFIG.cardPadX,
		cardPadY: cards.cardPadY ?? layout.cardPadY ?? DEFAULT_CARD_CONFIG.cardPadY,
	};
}

const validateThemeJson = Compile(ThemeJsonSchema);

export type ThemeColor =
	| "accent"
	| "border"
	| "borderAccent"
	| "borderMuted"
	| "success"
	| "error"
	| "warning"
	| "muted"
	| "dim"
	| "text"
	| "thinkingText"
	| "userMessageText"
	| "customMessageText"
	| "customMessageLabel"
	| "toolTitle"
	| "toolOutput"
	| "mdHeading"
	| "mdLink"
	| "mdLinkUrl"
	| "mdCode"
	| "mdCodeBlock"
	| "mdCodeBlockBorder"
	| "mdQuote"
	| "mdQuoteBorder"
	| "mdHr"
	| "mdListBullet"
	| "toolDiffAdded"
	| "toolDiffRemoved"
	| "toolDiffContext"
	| "syntaxComment"
	| "syntaxKeyword"
	| "syntaxFunction"
	| "syntaxVariable"
	| "syntaxString"
	| "syntaxNumber"
	| "syntaxType"
	| "syntaxOperator"
	| "syntaxPunctuation"
	| "thinkingOff"
	| "thinkingMinimal"
	| "thinkingLow"
	| "thinkingMedium"
	| "thinkingHigh"
	| "thinkingXhigh"
	| "bashMode"
	| "editorFg"
	| "editorCursor"
	| "editorLineNumber";

export type ThemeBg =
	| "selectedBg"
	| "userMessageBg"
	| "customMessageBg"
	| "toolPendingBg"
	| "toolSuccessBg"
	| "toolErrorBg"
	| "toolDiffAddedBg"
	| "toolDiffRemovedBg"
	| "toolWarningBg"
	| "editorBg"
	| "editorSelection"
	| "statusBarBg"
	| "surfaceBg"
	| "cardBg"
	| "thinkingBg";

type ColorMode = "truecolor" | "256color";

export interface ModelBrand {
	color: string;
	emoji: string;
	nerd: string;
	unicode: string;
	ascii: string;
}

const NERD = {
	asterisk: "\u{F06C4}",
	flower: "\u{F024A}",
	dolphin: "\u{F18B4}",
	closeThick: "\u{F1398}",
	creation: "\u{F0674}",
	fire: "\u{F0238}",
	hexagram: "\u{F0AC9}",
	brain: "\u{F09D1}",
	infinity: "\u{F06E4}",
	triangle: "\u{F0536}",
	moonCrescent: "\u{F0F65}",
	robot: "\u{F06A9}",
} as const;

function modelBrandFor(provider: string, modelLabel?: string): ModelBrand {
	const lower = `${provider} ${modelLabel ?? ""}`.toLowerCase();
	if (
		lower.includes("claude") ||
		lower.includes("opus") ||
		lower.includes("sonnet") ||
		lower.includes("haiku") ||
		lower.includes("fable") ||
		lower.includes("mythos") ||
		lower.includes("anthropic")
	) {
		return { color: "#d08030", emoji: NERD.asterisk, nerd: NERD.asterisk, unicode: "✳", ascii: "C" };
	}
	if (lower.includes("mistral") || lower.includes("codestral") || lower.includes("devstral")) {
		return { color: "#f06030", emoji: NERD.fire, nerd: NERD.fire, unicode: "◧", ascii: "M" };
	}
	if (lower.includes("deepseek")) {
		return { color: "#005faf", emoji: "🐋", nerd: NERD.dolphin, unicode: "◗", ascii: "D" };
	}
	if (lower.includes("gemma")) {
		return { color: "#5098e8", emoji: NERD.creation, nerd: NERD.creation, unicode: "✧", ascii: "g" };
	}
	if (lower.includes("gemini") || lower.includes("google")) {
		return { color: "#4285f4", emoji: NERD.creation, nerd: NERD.creation, unicode: "✦", ascii: "G" };
	}
	if (lower.includes("qwen")) {
		return { color: "#875fff", emoji: NERD.hexagram, nerd: NERD.hexagram, unicode: "⬡", ascii: "Q" };
	}
	if (lower.includes("glm") || lower.includes("zhipu") || lower.includes("zai")) {
		return { color: "#00afaf", emoji: NERD.brain, nerd: NERD.brain, unicode: "◎", ascii: "Z" };
	}
	if (lower.includes("llama") || lower.includes("meta")) {
		return { color: "#0087ff", emoji: NERD.infinity, nerd: NERD.infinity, unicode: "∞", ascii: "L" };
	}
	if (lower.includes("minimax")) {
		return { color: "#ff4444", emoji: NERD.triangle, nerd: NERD.triangle, unicode: "▽", ascii: "I" };
	}
	if (lower.includes("grok") || lower.includes("xai") || lower.includes("groq")) {
		return { color: "#eeeeee", emoji: NERD.closeThick, nerd: NERD.closeThick, unicode: "✕", ascii: "X" };
	}
	if (lower.includes("moonshot") || lower.includes("kimi")) {
		return { color: "#aaaaaa", emoji: NERD.moonCrescent, nerd: NERD.moonCrescent, unicode: "☾", ascii: "K" };
	}
	if (lower.includes("gpt") || lower.includes("openai") || lower.includes("codex") || /^o[13](?:\b|-)/.test(lower)) {
		return { color: "#cccccc", emoji: NERD.flower, nerd: NERD.flower, unicode: "❁", ascii: "O" };
	}
	return { color: "#61afef", emoji: NERD.robot, nerd: NERD.robot, unicode: "◆", ascii: "?" };
}

function prefersAsciiGlyph(): boolean {
	const term = process.env.TERM?.toLowerCase() ?? "";
	return term === "dumb" || process.env.NO_COLOR === "1";
}

/**
 * Which glyph tier the terminal can render, best-to-worst.
 *
 * Detection order (first match wins):
 *   "emoji" — terminals known to render emoji-width glyphs correctly
 *   "nerd"  — terminals known to support Nerd Font / Powerline symbols
 *   "unicode"— fallback: most terminals render ◆ ◦ ● etc. safely
 *   "ascii" — dumb terminals / NO_COLOR / explicit opt-down
 */
function detectGlyphTier(): "emoji" | "nerd" | "unicode" | "ascii" {
	// Explicit env overrides always win (opt-down or opt-up).
	if (process.env.HAMR_EMOJI_MODEL_GLYPHS === "1") return "emoji";
	if (process.env.HAMR_NERD_FONT === "1") return "nerd";

	if (prefersAsciiGlyph()) return "ascii";

	const termProgram = process.env.TERM_PROGRAM?.toLowerCase() ?? "";
	const term = process.env.TERM?.toLowerCase() ?? "";

	// Terminals with known-good emoji rendering.
	const emojiTerminals = new Set([
		"iterm2", // macOS iTerm2
		"apple_terminal", // macOS Terminal.app (>= 10.15)
		"kitty", // Kitty
		"ghostty", // Ghostty
		"wezterm", // WezTerm
		"warp", // Warp
		"tabby", // Tabby/Terminus
		"hyper", // Hyper.js
		"vscode", // VS Code integrated terminal
		"cursor", // Cursor IDE terminal
		"windsurf", // Windsurf IDE terminal
	]);
	if (emojiTerminals.has(termProgram)) return "emoji";

	// Check TERM for Kitty protocol.
	if (term.startsWith("xterm-kitty")) return "emoji";

	// Terminals that support Nerd Font / Powerline symbols.
	const nerdTerminals = new Set(["alacritty", "rio"]);
	if (nerdTerminals.has(termProgram)) return "nerd";

	// Truecolor terminals likely support at least unicode glyphs.
	const colorterm = process.env.COLORTERM?.toLowerCase() ?? "";
	if (colorterm === "truecolor" || colorterm === "24bit") {
		// Most truecolor terminals also support nerd/emoji, but be conservative.
		return "nerd";
	}

	return "unicode";
}

function getGlyphTier(): "emoji" | "nerd" | "unicode" {
	const tier = detectGlyphTier();
	return tier === "ascii" ? "unicode" : tier;
}

// ============================================================================
// Color Utilities
// ============================================================================

function hexToRgb(hex: string): { r: number; g: number; b: number } {
	const cleaned = hex.replace("#", "");
	if (cleaned.length !== 6) {
		throw new Error(`Invalid hex color: ${hex}`);
	}
	const r = parseInt(cleaned.substring(0, 2), 16);
	const g = parseInt(cleaned.substring(2, 4), 16);
	const b = parseInt(cleaned.substring(4, 6), 16);
	if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) {
		throw new Error(`Invalid hex color: ${hex}`);
	}
	return { r, g, b };
}

// The 6x6x6 color cube channel values (indices 0-5)
const CUBE_VALUES = [0, 95, 135, 175, 215, 255];

// Grayscale ramp values (indices 232-255, 24 grays from 8 to 238)
const GRAY_VALUES = Array.from({ length: 24 }, (_, i) => 8 + i * 10);

function findClosestCubeIndex(value: number): number {
	let minDist = Infinity;
	let minIdx = 0;
	for (let i = 0; i < CUBE_VALUES.length; i++) {
		const dist = Math.abs(value - CUBE_VALUES[i]);
		if (dist < minDist) {
			minDist = dist;
			minIdx = i;
		}
	}
	return minIdx;
}

function findClosestGrayIndex(gray: number): number {
	let minDist = Infinity;
	let minIdx = 0;
	for (let i = 0; i < GRAY_VALUES.length; i++) {
		const dist = Math.abs(gray - GRAY_VALUES[i]);
		if (dist < minDist) {
			minDist = dist;
			minIdx = i;
		}
	}
	return minIdx;
}

function colorDistance(r1: number, g1: number, b1: number, r2: number, g2: number, b2: number): number {
	// Weighted Euclidean distance (human eye is more sensitive to green)
	const dr = r1 - r2;
	const dg = g1 - g2;
	const db = b1 - b2;
	return dr * dr * 0.299 + dg * dg * 0.587 + db * db * 0.114;
}

function rgbTo256(r: number, g: number, b: number): number {
	// Find closest color in the 6x6x6 cube
	const rIdx = findClosestCubeIndex(r);
	const gIdx = findClosestCubeIndex(g);
	const bIdx = findClosestCubeIndex(b);
	const cubeR = CUBE_VALUES[rIdx];
	const cubeG = CUBE_VALUES[gIdx];
	const cubeB = CUBE_VALUES[bIdx];
	const cubeIndex = 16 + 36 * rIdx + 6 * gIdx + bIdx;
	const cubeDist = colorDistance(r, g, b, cubeR, cubeG, cubeB);

	// Find closest grayscale
	const gray = Math.round(0.299 * r + 0.587 * g + 0.114 * b);
	const grayIdx = findClosestGrayIndex(gray);
	const grayValue = GRAY_VALUES[grayIdx];
	const grayIndex = 232 + grayIdx;
	const grayDist = colorDistance(r, g, b, grayValue, grayValue, grayValue);

	// Check if color has noticeable saturation (hue matters)
	// If max-min spread is significant, prefer cube to preserve tint
	const maxC = Math.max(r, g, b);
	const minC = Math.min(r, g, b);
	const spread = maxC - minC;

	// Only consider grayscale if color is nearly neutral (spread < 10)
	// AND grayscale is actually closer
	if (spread < 10 && grayDist < cubeDist) {
		return grayIndex;
	}

	return cubeIndex;
}

function hexTo256(hex: string): number {
	const { r, g, b } = hexToRgb(hex);
	return rgbTo256(r, g, b);
}

function fgAnsi(color: string | number, mode: ColorMode): string {
	if (color === "") return "\x1b[39m";
	if (typeof color === "number") return `\x1b[38;5;${color}m`;
	if (color.startsWith("#")) {
		if (mode === "truecolor") {
			const { r, g, b } = hexToRgb(color);
			return `\x1b[38;2;${r};${g};${b}m`;
		} else {
			const index = hexTo256(color);
			return `\x1b[38;5;${index}m`;
		}
	}
	throw new Error(`Invalid color value: ${color}`);
}

function bgAnsi(color: string | number, mode: ColorMode): string {
	if (color === "") return "\x1b[49m";
	if (typeof color === "number") return `\x1b[48;5;${color}m`;
	if (color.startsWith("#")) {
		if (mode === "truecolor") {
			const { r, g, b } = hexToRgb(color);
			return `\x1b[48;2;${r};${g};${b}m`;
		} else {
			const index = hexTo256(color);
			return `\x1b[48;5;${index}m`;
		}
	}
	throw new Error(`Invalid color value: ${color}`);
}

function resolveVarRefs(
	value: ColorValue,
	vars: Record<string, ColorValue>,
	visited = new Set<string>(),
): string | number {
	if (typeof value === "number" || value === "" || value.startsWith("#")) {
		return value;
	}
	if (visited.has(value)) {
		throw new Error(`Circular variable reference detected: ${value}`);
	}
	if (!(value in vars)) {
		throw new Error(`Variable reference not found: ${value}`);
	}
	visited.add(value);
	return resolveVarRefs(vars[value], vars, visited);
}

function resolveColorRefs(
	value: ColorValue,
	colors: Record<string, ColorValue>,
	vars: Record<string, ColorValue>,
	visited = new Set<string>(),
): string | number {
	if (typeof value === "number" || value === "" || value.startsWith("#")) {
		return value;
	}
	if (value in vars) {
		return resolveVarRefs(vars[value], vars);
	}
	if (value in colors) {
		if (visited.has(value)) {
			throw new Error(`Circular color reference detected: ${value}`);
		}
		visited.add(value);
		return resolveColorRefs(colors[value], colors, vars, visited);
	}
	throw new Error(`Variable or color reference not found: ${value}`);
}

function resolveThemeColors<T extends Record<string, ColorValue>>(
	colors: T,
	vars: Record<string, ColorValue> = {},
): Record<keyof T, string | number> {
	const resolved: Record<string, string | number> = {};
	for (const [key, value] of Object.entries(colors)) {
		resolved[key] = resolveColorRefs(value, colors, vars);
	}
	return resolved as Record<keyof T, string | number>;
}

// ============================================================================
// Theme Class
// ============================================================================

export class Theme {
	readonly name?: string;
	readonly sourcePath?: string;
	sourceInfo?: SourceInfo;
	readonly modelAdaptive: boolean;
	readonly cards: CardConfig;
	private fgColors: Map<ThemeColor, string>;
	private bgColors: Map<ThemeBg, string>;
	private mode: ColorMode;

	constructor(
		fgColors: Record<ThemeColor, string | number>,
		bgColors: Record<ThemeBg, string | number>,
		mode: ColorMode,
		options: {
			name?: string;
			sourcePath?: string;
			sourceInfo?: SourceInfo;
			modelAdaptive?: boolean;
			cards?: CardConfig;
		} = {},
	) {
		this.name = options.name;
		this.sourcePath = options.sourcePath;
		this.sourceInfo = options.sourceInfo;
		this.modelAdaptive = options.modelAdaptive ?? true;
		this.cards = options.cards ?? DEFAULT_CARD_CONFIG;
		this.mode = mode;
		this.fgColors = new Map();
		for (const [key, value] of Object.entries(fgColors) as [ThemeColor, string | number][]) {
			this.fgColors.set(key, fgAnsi(value, mode));
		}
		this.bgColors = new Map();
		for (const [key, value] of Object.entries(bgColors) as [ThemeBg, string | number][]) {
			this.bgColors.set(key, bgAnsi(value, mode));
		}
	}

	fg(color: ThemeColor, text: string): string {
		const ansi = this.fgColors.get(color);
		if (!ansi) throw new Error(`Unknown theme color: ${color}`);
		return `${ansi}${text}\x1b[39m`; // Reset only foreground color
	}

	bg(color: ThemeBg, text: string): string {
		const ansi = this.bgColors.get(color);
		if (!ansi) throw new Error(`Unknown theme background color: ${color}`);
		return `${ansi}${text}\x1b[49m`; // Reset only background color
	}

	bold(text: string): string {
		return chalk.bold(text);
	}

	italic(text: string): string {
		return chalk.italic(text);
	}

	underline(text: string): string {
		return chalk.underline(text);
	}

	inverse(text: string): string {
		return chalk.inverse(text);
	}

	strikethrough(text: string): string {
		return chalk.strikethrough(text);
	}

	getFgAnsi(color: ThemeColor): string {
		const ansi = this.fgColors.get(color);
		if (!ansi) throw new Error(`Unknown theme color: ${color}`);
		return ansi;
	}

	getBgAnsi(color: ThemeBg): string {
		const ansi = this.bgColors.get(color);
		if (!ansi) throw new Error(`Unknown theme background color: ${color}`);
		return ansi;
	}

	getColorMode(): ColorMode {
		return this.mode;
	}

	getThinkingBorderColor(level: "off" | "minimal" | "low" | "medium" | "high" | "xhigh"): (str: string) => string {
		// Map thinking levels to dedicated theme colors
		switch (level) {
			case "off":
				return (str: string) => this.fg("thinkingOff", str);
			case "minimal":
				return (str: string) => this.fg("thinkingMinimal", str);
			case "low":
				return (str: string) => this.fg("thinkingLow", str);
			case "medium":
				return (str: string) => this.fg("thinkingMedium", str);
			case "high":
				return (str: string) => this.fg("thinkingHigh", str);
			case "xhigh":
				return (str: string) => this.fg("thinkingXhigh", str);
			default:
				return (str: string) => this.fg("thinkingOff", str);
		}
	}

	getBashModeBorderColor(): (str: string) => string {
		return (str: string) => this.fg("bashMode", str);
	}

	modelBrand(provider: string, modelLabel?: string): ModelBrand {
		return modelBrandFor(provider, modelLabel);
	}

	modelGlyph(provider: string, modelLabel?: string): string {
		const brand = this.modelBrand(provider, modelLabel);
		const tier = getGlyphTier();
		if (tier === "emoji") return brand.emoji;
		if (tier === "nerd") return brand.nerd;
		return brand.unicode;
	}

	/**
	 * Editor border color derived from model brand hex × thinking brightness.
	 * Mirrors synax's promptBoxAccent(): model family color dimmed/brightened
	 * by thinking level so the editor accent reflects the active model.
	 *
	 * Returns undefined when modelAdaptive is false — callers should fall
	 * back to getThinkingBorderColor() in that case.
	 */
	getModelEditorBorderColor(
		provider: string,
		modelId: string | undefined,
		thinkingLevel: string | undefined,
	): ((str: string) => string) | undefined {
		if (!this.modelAdaptive) return undefined;
		const hex = this.modelHexColor(provider, modelId);
		if (!hex) return undefined;

		// Brightness multipliers per thinking level (from synax)
		const mult =
			thinkingLevel === "xhigh"
				? 1.0
				: thinkingLevel === "high"
					? 0.85
					: thinkingLevel === "medium"
						? 0.65
						: thinkingLevel === "low"
							? 0.45
							: 0.3; // off / default

		const r = Math.round(parseInt(hex.slice(1, 3), 16) * mult);
		const g = Math.round(parseInt(hex.slice(3, 5), 16) * mult);
		const b = Math.round(parseInt(hex.slice(5, 7), 16) * mult);

		if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return undefined;

		const ansi = `\x1b[38;2;${r};${g};${b}m`;
		return (str: string) => `${ansi}${str}\x1b[39m`;
	}

	/**
	 * Brand accent color for a model provider + label.
	 * Mirrors synax's modelBrand() palette so each model family gets a
	 * distinct, readable accent on dark terminals.
	 *   Anthropic  → orange  #d08030   (claude, haiku, sonnet, opus, fable, mythos)
	 *   Mistral    → flame   #f06030   (mistral, codestral, devstral)
	 *   DeepSeek   → navy    #005faf
	 *   Gemma      → mid blue#5098e8   (check BEFORE gemini — same brand)
	 *   Gemini     → royal   #4285f4
	 *   Qwen       → purple  #875fff
	 *   GLM/Zhipu  → teal    #00afaf
	 *   Meta       → meta    #0087ff   (llama, meta)
	 *   MiniMax    → red     #ff4444
	 *   xAI        → white   #eeeeee   (grok, xai)
	 *   Moonshot   → silver  #aaaaaa   (kimi, moonshot)
	 *   OpenAI     → white   #cccccc   (gpt, o1, o3, openai)
	 *   fallback   → blue    #61afef
	 */
	modelColor(provider: string, modelLabel?: string): string {
		if (!this.modelAdaptive) return this.getFgAnsi("text");
		const label = modelLabel?.toLowerCase() ?? "";
		const prov = provider.toLowerCase();

		// Model-label-based detection (most precise)
		if (label) {
			// Anthropic — claude, haiku, sonnet, opus, fable, mythos
			if (
				label.includes("claude") ||
				label.includes("haiku") ||
				label.includes("sonnet") ||
				label.includes("opus") ||
				label.includes("fable") ||
				label.includes("mythos")
			) {
				return fgAnsi("#d08030", this.mode);
			}
			// Mistral — includes codestral, devstral
			if (label.includes("mistral") || label.includes("codestral") || label.includes("devstral")) {
				return fgAnsi("#f06030", this.mode);
			}
			if (label.includes("deepseek")) {
				return fgAnsi("#005faf", this.mode);
			}
			// Gemma BEFORE gemini (gemma is a substring of gemini)
			if (label.includes("gemma")) {
				return fgAnsi("#5098e8", this.mode);
			}
			if (label.includes("gemini")) {
				return fgAnsi("#4285f4", this.mode);
			}
			if (label.includes("qwen")) {
				return fgAnsi("#875fff", this.mode);
			}
			// GLM (Zhipu)
			if (label.includes("glm")) {
				return fgAnsi("#00afaf", this.mode);
			}
			// Meta / Llama
			if (label.includes("llama") || label.includes("meta")) {
				return fgAnsi("#0087ff", this.mode);
			}
			if (label.includes("minimax")) {
				return fgAnsi("#ff4444", this.mode);
			}
			// xAI — grok, xai
			if (label.includes("grok") || label.includes("xai")) {
				return fgAnsi("#eeeeee", this.mode);
			}
			// Moonshot — kimi, moonshot
			if (label.includes("moonshot") || label.includes("kimi")) {
				return fgAnsi("#aaaaaa", this.mode);
			}
			// OpenAI — gpt, o1, o3, openai
			if (label.includes("gpt") || label.includes("openai") || label.startsWith("o1") || label.startsWith("o3")) {
				return fgAnsi("#cccccc", this.mode);
			}
		}

		// Provider-based fallback when label doesn't match
		if (prov === "anthropic") {
			return fgAnsi("#d08030", this.mode);
		}
		if (prov === "openai") {
			return fgAnsi("#cccccc", this.mode);
		}
		if (prov === "google" || prov === "gemini") {
			return fgAnsi("#4285f4", this.mode);
		}
		if (prov === "mistral" || prov === "codestral" || prov === "devstral") {
			return fgAnsi("#f06030", this.mode);
		}
		if (prov === "deepseek") {
			return fgAnsi("#005faf", this.mode);
		}
		if (prov === "groq") {
			return fgAnsi("#eeeeee", this.mode);
		}
		if (prov === "moonshot" || prov === "kimi") {
			return fgAnsi("#aaaaaa", this.mode);
		}

		return fgAnsi("#61afef", this.mode);
	}

	/**
	 * Returns the hex color (without ANSI wrapping) for a model's brand identity.
	 * Mirrors modelColor() hex lookups but returns raw hex for use in custom
	 * styling (editor borders, per-message accents, etc.).
	 */
	modelHexColor(provider: string, modelLabel?: string): string | undefined {
		if (!this.modelAdaptive) return undefined;
		const label = modelLabel?.toLowerCase() ?? "";
		const prov = provider.toLowerCase();

		if (label) {
			if (
				label.includes("claude") ||
				label.includes("haiku") ||
				label.includes("sonnet") ||
				label.includes("opus") ||
				label.includes("fable") ||
				label.includes("mythos")
			) {
				return "#d08030";
			}
			if (label.includes("mistral") || label.includes("codestral") || label.includes("devstral")) {
				return "#f06030";
			}
			if (label.includes("deepseek")) {
				return "#005faf";
			}
			if (label.includes("gemma")) {
				return "#5098e8";
			}
			if (label.includes("gemini")) {
				return "#4285f4";
			}
			if (label.includes("qwen")) {
				return "#875fff";
			}
			if (label.includes("glm")) {
				return "#00afaf";
			}
			if (label.includes("llama") || label.includes("meta")) {
				return "#0087ff";
			}
			if (label.includes("minimax")) {
				return "#ff4444";
			}
			if (label.includes("grok") || label.includes("xai")) {
				return "#eeeeee";
			}
			if (label.includes("moonshot") || label.includes("kimi")) {
				return "#aaaaaa";
			}
			if (label.includes("gpt") || label.includes("openai") || label.startsWith("o1") || label.startsWith("o3")) {
				return "#cccccc";
			}
		}

		if (prov === "anthropic") return "#d08030";
		if (prov === "openai") return "#cccccc";
		if (prov === "google" || prov === "gemini") return "#4285f4";
		if (prov === "mistral" || prov === "codestral" || prov === "devstral") return "#f06030";
		if (prov === "deepseek") return "#005faf";
		if (prov === "groq") return "#eeeeee";
		if (prov === "moonshot" || prov === "kimi") return "#aaaaaa";

		return "#61afef";
	}
}

// ============================================================================
// Theme Loading
// ============================================================================

let BUILTIN_THEMES: Record<string, ThemeJson> | undefined;

function getBuiltinThemes(): Record<string, ThemeJson> {
	if (!BUILTIN_THEMES) {
		const themesDir = getThemesDir();
		const read = (file: string) => JSON.parse(fs.readFileSync(path.join(themesDir, file), "utf-8")) as ThemeJson;
		BUILTIN_THEMES = {
			hamr: read("hamr.json"),
			dark: read("dark.json"),
			light: read("light.json"),
			kawaii: read("kawaii.json"),
			pinkOut: read("pinkOut.json"),
		};
	}
	return BUILTIN_THEMES;
}

export function getAvailableThemes(): string[] {
	return getAvailableThemesWithPaths().map(({ name }) => name);
}

export interface ThemeInfo {
	name: string;
	path: string | undefined;
}

export function getAvailableThemesWithPaths(): ThemeInfo[] {
	const themesDir = getThemesDir();
	const result: ThemeInfo[] = [];
	const seen = new Set<string>();
	const addTheme = (themeInfo: ThemeInfo) => {
		if (seen.has(themeInfo.name)) {
			return;
		}
		seen.add(themeInfo.name);
		result.push(themeInfo);
	};

	// Built-in themes
	for (const name of Object.keys(getBuiltinThemes())) {
		addTheme({ name, path: path.join(themesDir, `${name}.json`) });
	}

	// Custom themes
	for (const themeInfo of getCustomThemeInfos()) {
		addTheme(themeInfo);
	}

	for (const [name, theme] of registeredThemes.entries()) {
		addTheme({ name, path: theme.sourcePath });
	}

	return result.sort((a, b) => a.name.localeCompare(b.name));
}

function getCustomThemeInfos(): ThemeInfo[] {
	const customThemesDir = getCustomThemesDir();
	const result: ThemeInfo[] = [];
	if (!fs.existsSync(customThemesDir)) {
		return result;
	}

	for (const file of fs.readdirSync(customThemesDir)) {
		if (!file.endsWith(".json")) {
			continue;
		}
		const themePath = path.join(customThemesDir, file);
		try {
			const customTheme = loadThemeFromPath(themePath);
			if (customTheme.name) {
				result.push({ name: customTheme.name, path: themePath });
			}
		} catch {
			// Invalid themes are ignored here; the resource loader reports them
			// during normal startup/reload.
		}
	}
	return result;
}

function parseThemeJson(label: string, json: unknown): ThemeJson {
	if (!validateThemeJson.Check(json)) {
		const errors = Array.from(validateThemeJson.Errors(json));
		const missingColors = new Set<string>();
		const otherErrors: string[] = [];

		for (const error of errors) {
			if (error.keyword === "required" && error.instancePath === "/colors") {
				const requiredProperties = (error.params as { requiredProperties?: string[] }).requiredProperties;
				for (const requiredProperty of requiredProperties ?? []) {
					missingColors.add(requiredProperty);
				}
				continue;
			}

			const path = error.instancePath || "/";
			otherErrors.push(`  - ${path}: ${error.message}`);
		}

		let errorMessage = `Invalid theme "${label}":\n`;
		if (missingColors.size > 0) {
			errorMessage += "\nMissing required color tokens:\n";
			errorMessage += Array.from(missingColors)
				.sort()
				.map((color) => `  - ${color}`)
				.join("\n");
			errorMessage += '\n\nPlease add these colors to your theme\'s "colors" object.';
			errorMessage += "\nSee the built-in themes (dark.json, light.json) for reference values.";
		}
		if (otherErrors.length > 0) {
			errorMessage += `\n\nOther errors:\n${otherErrors.join("\n")}`;
		}

		throw new Error(errorMessage);
	}

	return json as ThemeJson;
}

function parseThemeJsonContent(label: string, content: string): ThemeJson {
	let json: unknown;
	try {
		json = JSON.parse(content);
	} catch (error) {
		throw new Error(`Failed to parse theme ${label}: ${error}`);
	}
	return parseThemeJson(label, json);
}

function loadThemeJson(name: string): ThemeJson {
	const builtinThemes = getBuiltinThemes();
	if (name in builtinThemes) {
		return builtinThemes[name];
	}
	const registeredTheme = registeredThemes.get(name);
	if (registeredTheme?.sourcePath) {
		const content = fs.readFileSync(registeredTheme.sourcePath, "utf-8");
		return parseThemeJsonContent(registeredTheme.sourcePath, content);
	}
	if (registeredTheme) {
		throw new Error(`Theme "${name}" does not have a source path for export`);
	}
	const customThemesDir = getCustomThemesDir();
	const themePath = path.join(customThemesDir, `${name}.json`);
	if (!fs.existsSync(themePath)) {
		throw new Error(`Theme not found: ${name}`);
	}
	const content = fs.readFileSync(themePath, "utf-8");
	return parseThemeJsonContent(name, content);
}

function createTheme(themeJson: ThemeJson, mode?: ColorMode, sourcePath?: string): Theme {
	const colorMode = mode ?? (getCapabilities().trueColor ? "truecolor" : "256color");
	const resolvedColors = resolveThemeColors(themeJson.colors, themeJson.vars);
	const fgColors: Record<ThemeColor, string | number> = {} as Record<ThemeColor, string | number>;
	const bgColors: Record<ThemeBg, string | number> = {} as Record<ThemeBg, string | number>;
	const bgColorKeys: Set<string> = new Set([
		"selectedBg",
		"userMessageBg",
		"customMessageBg",
		"toolPendingBg",
		"toolSuccessBg",
		"toolErrorBg",
		"toolDiffAddedBg",
		"toolDiffRemovedBg",
		"toolWarningBg",
		"editorBg",
		"editorSelection",
		"statusBarBg",
		"surfaceBg",
		"cardBg",
		"thinkingBg",
	]);
	for (const [key, value] of Object.entries(resolvedColors)) {
		if (bgColorKeys.has(key)) {
			bgColors[key as ThemeBg] = value;
		} else {
			fgColors[key as ThemeColor] = value;
		}
	}
	return new Theme(fgColors, bgColors, colorMode, {
		name: themeJson.name,
		sourcePath,
		modelAdaptive: themeJson.modelAdaptive,
		cards: resolveCardConfig(themeJson),
	});
}

export function loadThemeFromPath(themePath: string, mode?: ColorMode): Theme {
	const content = fs.readFileSync(themePath, "utf-8");
	const themeJson = parseThemeJsonContent(themePath, content);
	return createTheme(themeJson, mode, themePath);
}

function loadTheme(name: string, mode?: ColorMode): Theme {
	const registeredTheme = registeredThemes.get(name);
	if (registeredTheme) {
		return registeredTheme;
	}
	const themeJson = loadThemeJson(name);
	return createTheme(themeJson, mode);
}

export function getThemeByName(name: string): Theme | undefined {
	try {
		return loadTheme(name);
	} catch {
		return undefined;
	}
}

export type TerminalTheme = "dark" | "light";

export interface TerminalThemeDetection {
	theme: TerminalTheme;
	source: "terminal background" | "COLORFGBG" | "fallback";
	detail: string;
	confidence: "high" | "low";
}

export interface TerminalThemeDetectionOptions {
	env?: NodeJS.ProcessEnv;
}

export interface TerminalBackgroundThemeDetector {
	queryTerminalBackgroundColor({ timeoutMs }: { timeoutMs: number }): Promise<RgbColor | undefined>;
}

export interface TerminalBackgroundThemeDetectionOptions extends TerminalThemeDetectionOptions {
	ui: TerminalBackgroundThemeDetector;
	timeoutMs: number;
}

function getColorFgBgBackgroundIndex(colorfgbg: string): number | undefined {
	const parts = colorfgbg.split(";");
	for (let i = parts.length - 1; i >= 0; i--) {
		const bg = parseInt(parts[i].trim(), 10);
		if (Number.isInteger(bg) && bg >= 0 && bg <= 255) {
			return bg;
		}
	}
	return undefined;
}

function getRgbColorLuminance({ r, g, b }: RgbColor): number {
	const toLinear = (channel: number) => {
		const value = channel / 255;
		return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
	};
	return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b);
}

function getAnsiColorLuminance(index: number): number {
	return getRgbColorLuminance(hexToRgb(ansi256ToHex(index)));
}

export function getThemeForRgbColor(rgb: RgbColor): TerminalTheme {
	return getRgbColorLuminance(rgb) >= 0.5 ? "light" : "dark";
}

export function detectTerminalBackgroundFromEnv(options: TerminalThemeDetectionOptions = {}): TerminalThemeDetection {
	const env = options.env ?? process.env;
	const colorfgbg = env.COLORFGBG || "";
	const bg = getColorFgBgBackgroundIndex(colorfgbg);
	if (bg !== undefined) {
		return {
			theme: getAnsiColorLuminance(bg) >= 0.5 ? "light" : "dark",
			source: "COLORFGBG",
			detail: `background color index ${bg}`,
			confidence: "high",
		};
	}

	return {
		theme: "dark",
		source: "fallback",
		detail: "no terminal background hint found",
		confidence: "low",
	};
}

export async function detectTerminalBackgroundTheme({
	ui,
	timeoutMs,
	env,
}: TerminalBackgroundThemeDetectionOptions): Promise<TerminalThemeDetection> {
	try {
		const rgb = await ui.queryTerminalBackgroundColor({ timeoutMs });
		if (rgb) {
			return {
				theme: getThemeForRgbColor(rgb),
				source: "terminal background",
				detail: `OSC 11 background rgb(${rgb.r}, ${rgb.g}, ${rgb.b})`,
				confidence: "high",
			};
		}
	} catch {
		// Fall back to environment-based detection when the terminal query fails.
	}

	return detectTerminalBackgroundFromEnv({ env });
}

export function getDefaultTheme(): string {
	return "hamr";
}

// ============================================================================
// Global Theme Instance
// ============================================================================

// Use globalThis to share theme across module loaders (tsx + jiti in dev mode)
const THEME_KEY = Symbol.for("@hamr/coding-agent:theme");
const THEME_KEY_OLD = Symbol.for("@mariozechner/pi-coding-agent:theme");

// Export theme as a getter that reads from globalThis
// This ensures all module instances (tsx, jiti) see the same theme
export const theme: Theme = new Proxy({} as Theme, {
	get(_target, prop) {
		const t = (globalThis as Record<symbol, Theme>)[THEME_KEY];
		if (!t) throw new Error("Theme not initialized. Call initTheme() first.");
		return (t as unknown as Record<string | symbol, unknown>)[prop];
	},
});

function setGlobalTheme(t: Theme): void {
	(globalThis as Record<symbol, Theme>)[THEME_KEY] = t;
	(globalThis as Record<symbol, Theme>)[THEME_KEY_OLD] = t;
}

let currentThemeName: string | undefined;
let themeWatcher: fs.FSWatcher | undefined;
let themeReloadTimer: NodeJS.Timeout | undefined;
let onThemeChangeCallback: (() => void) | undefined;
const registeredThemes = new Map<string, Theme>();

export function setRegisteredThemes(themes: Theme[]): void {
	registeredThemes.clear();
	for (const theme of themes) {
		if (theme.name) {
			registeredThemes.set(theme.name, theme);
		}
	}
}

export function initTheme(themeName?: string, enableWatcher: boolean = false): void {
	const name = themeName ?? getDefaultTheme();
	currentThemeName = name;
	try {
		setGlobalTheme(loadTheme(name));
		if (enableWatcher) {
			startThemeWatcher();
		}
	} catch (_error) {
		// Theme is invalid - fall back to hamr theme silently
		currentThemeName = "hamr";
		setGlobalTheme(loadTheme("hamr"));
		// Don't start watcher for fallback theme
	}
}

export function setTheme(name: string, enableWatcher: boolean = false): { success: boolean; error?: string } {
	currentThemeName = name;
	try {
		setGlobalTheme(loadTheme(name));
		if (enableWatcher) {
			startThemeWatcher();
		}
		if (onThemeChangeCallback) {
			onThemeChangeCallback();
		}
		return { success: true };
	} catch (error) {
		// Theme is invalid - fall back to hamr theme
		currentThemeName = "hamr";
		setGlobalTheme(loadTheme("hamr"));
		// Don't start watcher for fallback theme
		return {
			success: false,
			error: error instanceof Error ? error.message : String(error),
		};
	}
}

export function setThemeInstance(themeInstance: Theme): void {
	setGlobalTheme(themeInstance);
	currentThemeName = "<in-memory>";
	stopThemeWatcher(); // Can't watch a direct instance
	if (onThemeChangeCallback) {
		onThemeChangeCallback();
	}
}

export function onThemeChange(callback: () => void): void {
	onThemeChangeCallback = callback;
}

function startThemeWatcher(): void {
	stopThemeWatcher();

	// Only watch if it's a custom theme (not built-in)
	if (!currentThemeName || currentThemeName === "hamr") {
		return;
	}

	const customThemesDir = getCustomThemesDir();
	const watchedThemeName = currentThemeName;
	const watchedFileName = `${watchedThemeName}.json`;
	const themeFile = path.join(customThemesDir, watchedFileName);

	// Only watch if the file exists
	if (!fs.existsSync(themeFile)) {
		return;
	}

	const scheduleReload = () => {
		if (themeReloadTimer) {
			clearTimeout(themeReloadTimer);
		}
		themeReloadTimer = setTimeout(() => {
			themeReloadTimer = undefined;

			// Ignore stale timers after switching themes or stopping the watcher
			if (currentThemeName !== watchedThemeName) {
				return;
			}

			// Keep the last successfully loaded theme active if the file is temporarily missing
			if (!fs.existsSync(themeFile)) {
				return;
			}

			try {
				// Reload the theme from disk and refresh the registry cache
				const reloadedTheme = loadThemeFromPath(themeFile);
				registeredThemes.set(watchedThemeName, reloadedTheme);
				setGlobalTheme(reloadedTheme);
				// Notify callback (to invalidate UI)
				if (onThemeChangeCallback) {
					onThemeChangeCallback();
				}
			} catch (_error) {
				// Ignore errors (file might be in invalid state while being edited)
			}
		}, 100);
	};

	themeWatcher =
		watchWithErrorHandler(
			customThemesDir,
			(_eventType, filename) => {
				if (currentThemeName !== watchedThemeName) {
					return;
				}
				if (!filename) {
					scheduleReload();
					return;
				}
				if (filename !== watchedFileName) {
					return;
				}
				scheduleReload();
			},
			() => {
				closeWatcher(themeWatcher);
				themeWatcher = undefined;
			},
		) ?? undefined;
}

export function stopThemeWatcher(): void {
	if (themeReloadTimer) {
		clearTimeout(themeReloadTimer);
		themeReloadTimer = undefined;
	}
	closeWatcher(themeWatcher);
	themeWatcher = undefined;
}

// ============================================================================
// HTML Export Helpers
// ============================================================================

/**
 * Convert a 256-color index to hex string.
 * Indices 0-15: basic colors (approximate)
 * Indices 16-231: 6x6x6 color cube
 * Indices 232-255: grayscale ramp
 */
function ansi256ToHex(index: number): string {
	// Basic colors (0-15) - approximate common terminal values
	const basicColors = [
		"#000000",
		"#800000",
		"#008000",
		"#808000",
		"#000080",
		"#800080",
		"#008080",
		"#c0c0c0",
		"#808080",
		"#ff0000",
		"#00ff00",
		"#ffff00",
		"#0000ff",
		"#ff00ff",
		"#00ffff",
		"#ffffff",
	];
	if (index < 16) {
		return basicColors[index];
	}

	// Color cube (16-231): 6x6x6 = 216 colors
	if (index < 232) {
		const cubeIndex = index - 16;
		const r = Math.floor(cubeIndex / 36);
		const g = Math.floor((cubeIndex % 36) / 6);
		const b = cubeIndex % 6;
		const toHex = (n: number) => (n === 0 ? 0 : 55 + n * 40).toString(16).padStart(2, "0");
		return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
	}

	// Grayscale (232-255): 24 shades
	const gray = 8 + (index - 232) * 10;
	const grayHex = gray.toString(16).padStart(2, "0");
	return `#${grayHex}${grayHex}${grayHex}`;
}

/**
 * Get resolved theme colors as CSS-compatible hex strings.
 * Used by HTML export to generate CSS custom properties.
 */
export function getResolvedThemeColors(themeName?: string): Record<string, string> {
	const name = themeName ?? currentThemeName ?? getDefaultTheme();
	const isLight = name === "light";
	const themeJson = loadThemeJson(name);
	const resolved = resolveThemeColors(themeJson.colors, themeJson.vars);

	// Default text color for empty values (terminal uses default fg color)
	const defaultText = isLight ? "#000000" : "#e5e5e7";

	const cssColors: Record<string, string> = {};
	for (const [key, value] of Object.entries(resolved)) {
		if (typeof value === "number") {
			cssColors[key] = ansi256ToHex(value);
		} else if (value === "") {
			// Empty means default terminal color - use sensible fallback for HTML
			cssColors[key] = defaultText;
		} else {
			cssColors[key] = value;
		}
	}
	return cssColors;
}

/**
 * Check if a theme is a "light" theme (for CSS that needs light/dark variants).
 */
export function isLightTheme(themeName?: string): boolean {
	// Currently just check the name - could be extended to analyze colors
	return themeName === "light";
}

/**
 * Get explicit export colors from theme JSON, if specified.
 * Returns undefined for each color that isn't explicitly set.
 */
export function getThemeExportColors(themeName?: string): {
	pageBg?: string;
	cardBg?: string;
	infoBg?: string;
} {
	const name = themeName ?? currentThemeName ?? getDefaultTheme();
	try {
		const themeJson = loadThemeJson(name);
		const exportSection = themeJson.export;
		if (!exportSection) return {};

		const vars = themeJson.vars ?? {};
		const resolve = (value: ColorValue | undefined): string | undefined => {
			if (value === undefined) return undefined;
			const resolved = resolveVarRefs(value, vars);
			if (typeof resolved === "number") return ansi256ToHex(resolved);
			if (resolved === "") return undefined;
			return resolved;
		};

		return {
			pageBg: resolve(exportSection.pageBg),
			cardBg: resolve(exportSection.cardBg),
			infoBg: resolve(exportSection.infoBg),
		};
	} catch {
		return {};
	}
}

// ============================================================================
// TUI Helpers
// ============================================================================

type CliHighlightTheme = Record<string, (s: string) => string>;

let cachedHighlightThemeFor: Theme | undefined;
let cachedCliHighlightTheme: CliHighlightTheme | undefined;

function buildCliHighlightTheme(t: Theme): CliHighlightTheme {
	return {
		keyword: (s: string) => t.fg("syntaxKeyword", s),
		built_in: (s: string) => t.fg("syntaxType", s),
		literal: (s: string) => t.fg("syntaxNumber", s),
		number: (s: string) => t.fg("syntaxNumber", s),
		regexp: (s: string) => t.fg("syntaxString", s),
		string: (s: string) => t.fg("syntaxString", s),
		comment: (s: string) => t.fg("syntaxComment", s),
		doctag: (s: string) => t.fg("syntaxComment", s),
		meta: (s: string) => t.fg("muted", s),
		function: (s: string) => t.fg("syntaxFunction", s),
		title: (s: string) => t.fg("syntaxFunction", s),
		class: (s: string) => t.fg("syntaxType", s),
		type: (s: string) => t.fg("syntaxType", s),
		tag: (s: string) => t.fg("syntaxPunctuation", s),
		name: (s: string) => t.fg("syntaxKeyword", s),
		attr: (s: string) => t.fg("syntaxVariable", s),
		variable: (s: string) => t.fg("syntaxVariable", s),
		params: (s: string) => t.fg("syntaxVariable", s),
		operator: (s: string) => t.fg("syntaxOperator", s),
		punctuation: (s: string) => t.fg("syntaxPunctuation", s),
		emphasis: (s: string) => t.italic(s),
		strong: (s: string) => t.bold(s),
		link: (s: string) => t.underline(s),
		addition: (s: string) => t.fg("toolDiffAdded", s),
		deletion: (s: string) => t.fg("toolDiffRemoved", s),
	};
}

function getCliHighlightTheme(t: Theme): CliHighlightTheme {
	if (cachedHighlightThemeFor !== t || !cachedCliHighlightTheme) {
		cachedHighlightThemeFor = t;
		cachedCliHighlightTheme = buildCliHighlightTheme(t);
	}
	return cachedCliHighlightTheme;
}

/**
 * Highlight code with syntax coloring based on file extension or language.
 * Returns array of highlighted lines.
 */
export function highlightCode(code: string, lang?: string): string[] {
	// Validate language before highlighting to avoid stderr spam from cli-highlight
	const validLang = lang && supportsLanguage(lang) ? lang : undefined;
	// Skip highlighting when no valid language is specified. cli-highlight's
	// auto-detection is unreliable and can misidentify prose as AppleScript,
	// LiveCodeServer, etc., coloring random English words as keywords.
	if (!validLang) {
		return code.split("\n").map((line) => theme.fg("mdCodeBlock", line));
	}
	const opts = {
		language: validLang,
		ignoreIllegals: true,
		theme: getCliHighlightTheme(theme),
	};
	try {
		return highlight(code, opts).split("\n");
	} catch {
		return code.split("\n");
	}
}

/**
 * Get language identifier from file path extension.
 */
export function getLanguageFromPath(filePath: string): string | undefined {
	const ext = filePath.split(".").pop()?.toLowerCase();
	if (!ext) return undefined;

	const extToLang: Record<string, string> = {
		ts: "typescript",
		tsx: "typescript",
		js: "javascript",
		jsx: "javascript",
		mjs: "javascript",
		cjs: "javascript",
		py: "python",
		rb: "ruby",
		rs: "rust",
		go: "go",
		java: "java",
		kt: "kotlin",
		swift: "swift",
		c: "c",
		h: "c",
		cpp: "cpp",
		cc: "cpp",
		cxx: "cpp",
		hpp: "cpp",
		cs: "csharp",
		php: "php",
		sh: "bash",
		bash: "bash",
		zsh: "bash",
		fish: "fish",
		ps1: "powershell",
		sql: "sql",
		html: "html",
		htm: "html",
		css: "css",
		scss: "scss",
		sass: "sass",
		less: "less",
		json: "json",
		yaml: "yaml",
		yml: "yaml",
		toml: "toml",
		xml: "xml",
		md: "markdown",
		markdown: "markdown",
		dockerfile: "dockerfile",
		makefile: "makefile",
		cmake: "cmake",
		lua: "lua",
		perl: "perl",
		r: "r",
		scala: "scala",
		clj: "clojure",
		ex: "elixir",
		exs: "elixir",
		erl: "erlang",
		hs: "haskell",
		ml: "ocaml",
		vim: "vim",
		graphql: "graphql",
		proto: "protobuf",
		tf: "hcl",
		hcl: "hcl",
	};

	return extToLang[ext];
}

export function getMarkdownTheme(): MarkdownTheme {
	return {
		heading: (text: string) => theme.fg("mdHeading", text),
		link: (text: string) => theme.fg("mdLink", text),
		linkUrl: (text: string) => theme.fg("mdLinkUrl", text),
		code: (text: string) => theme.fg("mdCode", text),
		codeBlock: (text: string) => theme.fg("mdCodeBlock", text),
		codeBlockBorder: (text: string) => theme.fg("mdCodeBlockBorder", text),
		quote: (text: string) => theme.fg("mdQuote", text),
		quoteBorder: (text: string) => theme.fg("mdQuoteBorder", text),
		hr: (text: string) => theme.fg("mdHr", text),
		listBullet: (text: string) => theme.fg("mdListBullet", text),
		bold: (text: string) => theme.bold(text),
		italic: (text: string) => theme.italic(text),
		underline: (text: string) => theme.underline(text),
		strikethrough: (text: string) => chalk.strikethrough(text),
		highlightCode: (code: string, lang?: string): string[] => {
			const rawLines = code.split("\n");
			// marked usually strips the trailing newline, but guard against an
			// empty final line so the gutter count stays accurate.
			while (rawLines.length > 1 && rawLines[rawLines.length - 1] === "") rawLines.pop();
			const gutterWidth = String(rawLines.length).length;
			const gutter = (n: number) => theme.fg("mdCodeBlockBorder", String(n).padStart(gutterWidth));

			// Diff/patch blocks: shade +/- lines red/green (with a line-number
			// gutter) instead of running them through the syntax highlighter.
			if (lang === "diff" || lang === "patch") {
				return rawLines.map((line, i) => {
					const numbered = `${gutter(i + 1)} ${line}`;
					if (line.startsWith("+") && !line.startsWith("+++")) {
						return theme.bg("toolDiffAddedBg", theme.fg("toolDiffAdded", numbered));
					}
					if (line.startsWith("-") && !line.startsWith("---")) {
						return theme.bg("toolDiffRemovedBg", theme.fg("toolDiffRemoved", numbered));
					}
					return `${gutter(i + 1)} ${theme.fg("toolDiffContext", line)}`;
				});
			}

			// Validate language before highlighting to avoid stderr spam from cli-highlight
			// and prose being misdetected as code (cli-highlight auto-detection is noisy).
			const validLang = lang && supportsLanguage(lang) ? lang : undefined;
			let highlighted: string[];
			if (!validLang) {
				highlighted = rawLines.map((line) => theme.fg("mdCodeBlock", line));
			} else {
				try {
					highlighted = highlight(code, {
						language: validLang,
						ignoreIllegals: true,
						theme: getCliHighlightTheme(theme),
					}).split("\n");
				} catch {
					highlighted = rawLines.map((line) => theme.fg("mdCodeBlock", line));
				}
			}
			// Prepend a dim line-number gutter to every code line.
			return rawLines.map((raw, i) => `${gutter(i + 1)} ${highlighted[i] ?? theme.fg("mdCodeBlock", raw)}`);
		},
	};
}

export function getSelectListTheme(): SelectListTheme {
	return {
		selectedPrefix: (text: string) => theme.fg("accent", text),
		selectedText: (text: string) => theme.fg("accent", text),
		description: (text: string) => theme.fg("muted", text),
		scrollInfo: (text: string) => theme.fg("muted", text),
		noMatch: (text: string) => theme.fg("muted", text),
	};
}

export function getEditorTheme(): EditorTheme {
	return {
		borderColor: (text: string) => theme.fg("borderMuted", text),
		selectList: getSelectListTheme(),
	};
}

export function getSettingsListTheme(): SettingsListTheme {
	return {
		label: (text: string, selected: boolean) => (selected ? theme.fg("accent", text) : text),
		value: (text: string, selected: boolean) => (selected ? theme.fg("accent", text) : theme.fg("muted", text)),
		description: (text: string) => theme.fg("dim", text),
		section: (text: string) => theme.bold(theme.fg("accent", text)),
		cursor: theme.fg("accent", "→ "),
		hint: (text: string) => theme.fg("dim", text),
	};
}
