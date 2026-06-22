import { type EditorTheme, type MarkdownTheme, type RgbColor, type SelectListTheme, type SettingsListTheme } from "@hamr/tui";
import type { SourceInfo } from "../../../core/source-info.ts";
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
    /** Left indent applied to tool card headings within the card Box. */
    toolIndent: number;
    /** Left indent applied to tool card results (visually separates result from heading). */
    toolResultIndent: number;
    /** Horizontal/vertical padding of the card Box. */
    cardPadX: number;
    cardPadY: number;
    /** Whether message/tool cards paint full-width background surfaces. */
    shadedSurfaces: boolean;
    /** Whether the THOUGHT card uses the dedicated thinking background when shaded surfaces are enabled. */
    thinkingShaded: boolean;
    /** When true, no spacer is inserted between consecutive cards. */
    gaplessCards: boolean;
    /** Whether to render the THOUGHT heading in thinking cards. */
    showThoughtHeading: boolean;
}
export declare const DEFAULT_CARD_CONFIG: CardConfig;
/**
 * Merge a theme JSON's `cards` (and legacy `layout`) blocks over the defaults.
 * `cards.cardPadX/Y` win over the legacy `layout.cardPadX/Y`.
 */
export declare function resolveCardConfig(json: {
    layout?: {
        cardPadX?: number;
        cardPadY?: number;
    };
    cards?: Partial<CardConfig>;
}): CardConfig;
export type ThemeColor = "accent" | "border" | "borderAccent" | "borderMuted" | "success" | "error" | "warning" | "muted" | "dim" | "text" | "thinkingText" | "userMessageText" | "customMessageText" | "customMessageLabel" | "toolTitle" | "toolOutput" | "mdHeading" | "mdLink" | "mdLinkUrl" | "mdCode" | "mdCodeBlock" | "mdCodeBlockBorder" | "mdQuote" | "mdQuoteBorder" | "mdHr" | "mdListBullet" | "toolDiffAdded" | "toolDiffRemoved" | "toolDiffContext" | "syntaxComment" | "syntaxKeyword" | "syntaxFunction" | "syntaxVariable" | "syntaxString" | "syntaxNumber" | "syntaxType" | "syntaxOperator" | "syntaxPunctuation" | "thinkingOff" | "thinkingMinimal" | "thinkingLow" | "thinkingMedium" | "thinkingHigh" | "thinkingXhigh" | "bashMode" | "editorFg" | "editorCursor" | "editorLineNumber";
export type ThemeBg = "selectedBg" | "userMessageBg" | "customMessageBg" | "toolPendingBg" | "toolSuccessBg" | "toolErrorBg" | "toolDiffAddedBg" | "toolDiffRemovedBg" | "toolWarningBg" | "editorBg" | "editorSelection" | "statusBarBg" | "surfaceBg" | "cardBg" | "thinkingBg";
type ColorMode = "truecolor" | "256color";
export interface ModelBrand {
    color: string;
    emoji: string;
    nerd: string;
    unicode: string;
    ascii: string;
}
export declare class Theme {
    readonly name?: string;
    readonly sourcePath?: string;
    sourceInfo?: SourceInfo;
    readonly modelAdaptive: boolean;
    readonly cards: CardConfig;
    private fgColors;
    private bgColors;
    private mode;
    constructor(fgColors: Record<ThemeColor, string | number>, bgColors: Record<ThemeBg, string | number>, mode: ColorMode, options?: {
        name?: string;
        sourcePath?: string;
        sourceInfo?: SourceInfo;
        modelAdaptive?: boolean;
        cards?: CardConfig;
    });
    fg(color: ThemeColor, text: string): string;
    bg(color: ThemeBg, text: string): string;
    bold(text: string): string;
    italic(text: string): string;
    underline(text: string): string;
    inverse(text: string): string;
    strikethrough(text: string): string;
    getFgAnsi(color: ThemeColor): string;
    getBgAnsi(color: ThemeBg): string;
    getColorMode(): ColorMode;
    getThinkingBorderColor(level: "off" | "minimal" | "low" | "medium" | "high" | "xhigh"): (str: string) => string;
    getBashModeBorderColor(): (str: string) => string;
    modelBrand(provider: string, modelLabel?: string): ModelBrand;
    modelGlyph(provider: string, modelLabel?: string): string;
    /**
     * Editor border color derived from model brand hex × thinking brightness.
     * Mirrors synax's promptBoxAccent(): model family color dimmed/brightened
     * by thinking level so the editor accent reflects the active model.
     *
     * Returns undefined when modelAdaptive is false — callers should fall
     * back to getThinkingBorderColor() in that case.
     */
    getModelEditorBorderColor(provider: string, modelId: string | undefined, thinkingLevel: string | undefined): ((str: string) => string) | undefined;
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
    modelColor(provider: string, modelLabel?: string): string;
    /**
     * Returns the hex color (without ANSI wrapping) for a model's brand identity.
     * Mirrors modelColor() hex lookups but returns raw hex for use in custom
     * styling (editor borders, per-message accents, etc.).
     */
    modelHexColor(provider: string, modelLabel?: string): string | undefined;
}
export declare function getAvailableThemes(): string[];
export interface ThemeInfo {
    name: string;
    path: string | undefined;
}
export declare function getAvailableThemesWithPaths(): ThemeInfo[];
export declare function loadThemeFromPath(themePath: string, mode?: ColorMode): Theme;
export declare function getThemeByName(name: string): Theme | undefined;
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
    queryTerminalBackgroundColor({ timeoutMs }: {
        timeoutMs: number;
    }): Promise<RgbColor | undefined>;
}
export interface TerminalBackgroundThemeDetectionOptions extends TerminalThemeDetectionOptions {
    ui: TerminalBackgroundThemeDetector;
    timeoutMs: number;
}
export declare function getThemeForRgbColor(rgb: RgbColor): TerminalTheme;
export declare function detectTerminalBackgroundFromEnv(options?: TerminalThemeDetectionOptions): TerminalThemeDetection;
export declare function detectTerminalBackgroundTheme({ ui, timeoutMs, env, }: TerminalBackgroundThemeDetectionOptions): Promise<TerminalThemeDetection>;
export declare function getDefaultTheme(): string;
export declare const theme: Theme;
export declare function setRegisteredThemes(themes: Theme[]): void;
export declare function initTheme(themeName?: string, enableWatcher?: boolean): void;
export declare function setTheme(name: string, enableWatcher?: boolean): {
    success: boolean;
    error?: string;
};
export declare function setThemeInstance(themeInstance: Theme): void;
export declare function onThemeChange(callback: () => void): void;
export declare function stopThemeWatcher(): void;
/**
 * Get resolved theme colors as CSS-compatible hex strings.
 * Used by HTML export to generate CSS custom properties.
 */
export declare function getResolvedThemeColors(themeName?: string): Record<string, string>;
/**
 * Check if a theme is a "light" theme (for CSS that needs light/dark variants).
 */
export declare function isLightTheme(themeName?: string): boolean;
/**
 * Get explicit export colors from theme JSON, if specified.
 * Returns undefined for each color that isn't explicitly set.
 */
export declare function getThemeExportColors(themeName?: string): {
    pageBg?: string;
    cardBg?: string;
    infoBg?: string;
};
/**
 * Highlight code with syntax coloring based on file extension or language.
 * Returns array of highlighted lines.
 */
export declare function highlightCode(code: string, lang?: string): string[];
/**
 * Get language identifier from file path extension.
 */
export declare function getLanguageFromPath(filePath: string): string | undefined;
export declare function getMarkdownTheme(): MarkdownTheme;
export declare function getSelectListTheme(): SelectListTheme;
export declare function getEditorTheme(): EditorTheme;
export declare function getSettingsListTheme(): SettingsListTheme;
export {};
//# sourceMappingURL=theme.d.ts.map