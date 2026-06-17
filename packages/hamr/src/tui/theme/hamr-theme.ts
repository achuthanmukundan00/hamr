import * as fs from 'node:fs';
import * as path from 'node:path';
import { detectCapabilities } from 'sexy-tui-rs';

// ── Color Tokens ────────────────────────────────────────────────────────────

export const THEME_COLORS = [
  'accent',
  'border',
  'borderAccent',
  'borderMuted',
  'success',
  'error',
  'warning',
  'muted',
  'dim',
  'text',
  'thinkingText',
  'selectedBg',
  'userMessageBg',
  'userMessageText',
  'toolPendingBg',
  'toolSuccessBg',
  'toolErrorBg',
  'toolWarningBg',
  'toolTitle',
  'toolOutput',
  'thinkingBg',
  'surfaceBg',
  'toolDiffAddedBg',
  'toolDiffRemovedBg',
  'cardBg',
  'statusBarBg',
  'editorBg',
  'editorFg',
  'editorCursor',
  'editorSelection',
  'editorLineNumber',
  'mdHeading',
  'mdLink',
  'mdLinkUrl',
  'mdCode',
  'mdCodeBlock',
  'mdCodeBlockBorder',
  'mdQuote',
  'mdQuoteBorder',
  'mdHr',
  'mdListBullet',
  'toolDiffAdded',
  'toolDiffRemoved',
  'toolDiffAddedBg',
  'toolDiffRemovedBg',
  'toolDiffContext',
  'syntaxComment',
  'syntaxKeyword',
  'syntaxFunction',
  'syntaxVariable',
  'syntaxString',
  'syntaxNumber',
  'syntaxType',
  'syntaxOperator',
  'syntaxPunctuation',
  // Pi cross-compatibility keys (optional in Hamr, required in Pi)
  'customMessageBg',
  'customMessageText',
  'customMessageLabel',
  'thinkingOff',
  'thinkingMinimal',
  'thinkingLow',
  'thinkingMedium',
  'thinkingHigh',
  'thinkingXhigh',
  'bashMode',
] as const;

export type ThemeColor = (typeof THEME_COLORS)[number];

type RawColorValue = string | number;
type ResolvedColor = string | number; // hex "#RRGGBB", number (256 idx), or "" (terminal default)

type ThemeJson = {
  name: string;
  vars?: Record<string, RawColorValue>;
  colors: Record<string, RawColorValue>;
  layout?: { cardPadX?: number; cardPadY?: number };
  modelAdaptive?: boolean;
};

export type ColorMode = 'truecolor' | '256' | 'dumb';

export const FG_COLORS: ThemeColor[] = [
  'accent',
  'border',
  'borderAccent',
  'borderMuted',
  'success',
  'error',
  'warning',
  'muted',
  'dim',
  'text',
  'thinkingText',
  'userMessageText',
  'toolTitle',
  'toolOutput',
  'editorFg',
  'editorCursor',
  'editorLineNumber',
  'mdHeading',
  'mdLink',
  'mdLinkUrl',
  'mdCode',
  'mdCodeBlock',
  'mdCodeBlockBorder',
  'mdQuote',
  'mdQuoteBorder',
  'mdHr',
  'mdListBullet',
  'toolDiffAdded',
  'toolDiffRemoved',
  'toolDiffContext',
  'syntaxComment',
  'syntaxKeyword',
  'syntaxFunction',
  'syntaxVariable',
  'syntaxString',
  'syntaxNumber',
  'syntaxType',
  'syntaxOperator',
  'syntaxPunctuation',
  'customMessageText',
  'customMessageLabel',
  'thinkingOff',
  'thinkingMinimal',
  'thinkingLow',
  'thinkingMedium',
  'thinkingHigh',
  'thinkingXhigh',
  'bashMode',
];

export const BG_COLORS: ThemeColor[] = [
  'selectedBg',
  'userMessageBg',
  'toolPendingBg',
  'toolSuccessBg',
  'toolErrorBg',
  'toolWarningBg',
  'thinkingBg',
  'surfaceBg',
  'toolDiffAddedBg',
  'toolDiffRemovedBg',
  'cardBg',
  'statusBarBg',
  'editorBg',
  'editorSelection',
  'customMessageBg',
];

// ── Color Utilities ──────────────────────────────────────────────────────────

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const cleaned = hex.replace('#', '');
  if (cleaned.length !== 6) throw new Error(`Invalid hex color: ${hex}`);
  const r = parseInt(cleaned.substring(0, 2), 16);
  const g = parseInt(cleaned.substring(2, 4), 16);
  const b = parseInt(cleaned.substring(4, 6), 16);
  if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) throw new Error(`Invalid hex color: ${hex}`);
  return { r, g, b };
}

const CUBE_VALUES = [0, 95, 135, 175, 215, 255];

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

function colorDistance(r1: number, g1: number, b1: number, r2: number, g2: number, b2: number): number {
  const dr = r1 - r2,
    dg = g1 - g2,
    db = b1 - b2;
  return dr * dr * 0.299 + dg * dg * 0.587 + db * db * 0.114;
}

function rgbTo256(r: number, g: number, b: number): number {
  const rIdx = findClosestCubeIndex(r);
  const gIdx = findClosestCubeIndex(g);
  const bIdx = findClosestCubeIndex(b);
  const cubeR = CUBE_VALUES[rIdx],
    cubeG = CUBE_VALUES[gIdx],
    cubeB = CUBE_VALUES[bIdx];
  const cubeIndex = 16 + 36 * rIdx + 6 * gIdx + bIdx;
  const cubeDist = colorDistance(r, g, b, cubeR, cubeG, cubeB);
  const gray = Math.round(0.299 * r + 0.587 * g + 0.114 * b);
  const grayClosest = Math.min(23, Math.max(0, Math.round((gray - 8) / 10)));
  const grayValue = 8 + grayClosest * 10;
  const grayIndex = 232 + grayClosest;
  const grayDist = colorDistance(r, g, b, grayValue, grayValue, grayValue);
  const spread = Math.max(r, g, b) - Math.min(r, g, b);
  if (spread < 10 && grayDist < cubeDist) return grayIndex;
  return cubeIndex;
}

function hexTo256(hex: string): number {
  const { r, g, b } = hexToRgb(hex);
  return rgbTo256(r, g, b);
}

function fgAnsi(color: ResolvedColor, mode: ColorMode): string {
  if (color === '') return '\x1b[39m';
  if (typeof color === 'number') return `\x1b[38;5;${color}m`;
  if (color.startsWith('#')) {
    if (mode === 'truecolor') {
      const { r, g, b } = hexToRgb(color);
      return `\x1b[38;2;${r};${g};${b}m`;
    }
    return `\x1b[38;5;${hexTo256(color)}m`;
  }
  throw new Error(`Invalid fg color value: ${color}`);
}

function bgAnsi(color: ResolvedColor, mode: ColorMode): string {
  if (color === '') return '\x1b[49m';
  if (typeof color === 'number') return `\x1b[48;5;${color}m`;
  if (color.startsWith('#')) {
    if (mode === 'truecolor') {
      const { r, g, b } = hexToRgb(color);
      return `\x1b[48;2;${r};${g};${b}m`;
    }
    return `\x1b[48;5;${hexTo256(color)}m`;
  }
  throw new Error(`Invalid bg color value: ${color}`);
}

function resolveVarRefs(
  value: RawColorValue,
  vars: Record<string, RawColorValue>,
  visited = new Set<string>(),
): ResolvedColor {
  if (typeof value === 'number' || value === '' || value.startsWith('#')) return value;
  if (visited.has(value)) throw new Error(`Circular variable reference: ${value}`);
  if (!(value in vars)) throw new Error(`Variable not found: ${value}`);
  visited.add(value);
  return resolveVarRefs(vars[value], vars, visited);
}

function resolveColors(
  colors: Record<string, RawColorValue>,
  vars: Record<string, RawColorValue> = {},
): Record<string, ResolvedColor> {
  const resolved: Record<string, ResolvedColor> = {};
  for (const [key, value] of Object.entries(colors)) {
    resolved[key] = resolveVarRefs(value, vars);
  }
  return resolved;
}

// ── Terminal Color Mode Detection ───────────────────────────────────────────

export function detectColorMode(): ColorMode {
  if (typeof process === 'undefined') return 'dumb';

  // Respect NO_COLOR and HAMR_ASCII
  if (process.env.NO_COLOR !== undefined || process.env.HAMR_ASCII === '1') return 'dumb';

  // Use terminal capability detection (via sexy-tui-rs, pi-tui derived)
  try {
    const caps = detectCapabilities();
    if (caps.trueColor) return 'truecolor';
    // 256 colors is the default for most modern terminals, assume it if not explicitly dumb
    return '256';
  } catch {
    // Fallback: check environment variables
    const colorterm = process.env.COLORTERM || '';
    const term = process.env.TERM || '';

    if (colorterm.includes('truecolor') || colorterm.includes('24bit')) return 'truecolor';
    if (colorterm.includes('256') || term.includes('256')) return '256';
    if (term) return 'dumb'; // TERM is set but no color support detected
    return 'dumb';
  }
}

// ── Theme Search Path ──────────────────────────────────────────────────────

function getBuiltInDir(): string {
  return __dirname;
}

function getUserThemesDir(): string {
  const home = process.env.HOME || process.env.USERPROFILE || '';
  return path.join(home, '.config', 'hamr', 'themes');
}

function findThemeFile(name: string, builtInDir: string, userDir: string): string | null {
  // Try user directory first (allows overriding built-ins)
  const userPath = path.join(userDir, name.endsWith('.json') ? name : `${name}.json`);
  if (fs.existsSync(userPath)) return userPath;

  const builtInPath = path.join(builtInDir, name.endsWith('.json') ? name : `${name}.json`);
  if (fs.existsSync(builtInPath)) return builtInPath;

  return null;
}

function listThemes(builtInDir: string, userDir: string): { name: string; path: string; builtIn: boolean }[] {
  const themes: { name: string; path: string; builtIn: boolean }[] = [];

  // Built-in themes
  if (fs.existsSync(builtInDir)) {
    for (const file of fs.readdirSync(builtInDir)) {
      if (file.endsWith('.json') && !file.startsWith('theme-schema')) {
        themes.push({ name: file.replace(/\.json$/, ''), path: path.join(builtInDir, file), builtIn: true });
      }
    }
  }

  // User themes (override built-ins with same name)
  if (fs.existsSync(userDir)) {
    for (const file of fs.readdirSync(userDir)) {
      if (file.endsWith('.json') && !file.startsWith('theme-schema')) {
        const name = file.replace(/\.json$/, '');
        const existing = themes.findIndex((t) => t.name === name);
        if (existing >= 0) themes[existing] = { name, path: path.join(userDir, file), builtIn: false };
        else themes.push({ name, path: path.join(userDir, file), builtIn: false });
      }
    }
  }

  return themes;
}

// ── Pi theme cross-compatibility ───────────────────────────────────────────

/**
 * Pi theme keys that Hamr doesn't natively support.  When a Pi theme is
 * loaded we derive Hamr equivalents from these so the theme "just works."
 */
const PI_ONLY_COLORS = new Set([
  'customMessageBg',
  'customMessageText',
  'customMessageLabel',
  'thinkingOff',
  'thinkingMinimal',
  'thinkingLow',
  'thinkingMedium',
  'thinkingHigh',
  'thinkingXhigh',
  'bashMode',
]);

/** Default Hamr→Pi key mappings used when a theme doesn't define Pi keys. */
const PI_DEFAULTS: Record<string, string> = {
  customMessageBg: 'toolPendingBg',
  customMessageText: 'text',
  customMessageLabel: 'accent',
  thinkingOff: 'dim',
  thinkingMinimal: 'thinkingText',
  thinkingLow: 'muted',
  thinkingMedium: 'accent',
  thinkingHigh: 'accent',
  thinkingXhigh: 'error',
  bashMode: 'accent',
};

/**
 * Hamr-only keys that Pi themes don't provide.  We derive sensible defaults
 * from the Pi colors that ARE present.
 */
const PI_TO_HAMR_FALLBACKS: Record<string, string> = {
  toolWarningBg: 'toolPendingBg',
  thinkingBg: 'toolPendingBg',
  surfaceBg: 'toolPendingBg',
  cardBg: 'userMessageBg',
  statusBarBg: 'userMessageBg',
  editorBg: 'userMessageBg',
  editorFg: 'text',
  editorCursor: 'accent',
  editorSelection: 'selectedBg',
  editorLineNumber: 'dim',
};

/**
 * Normalize a Pi theme's color map into a Hamr-compatible map by filling
 * in Hamr-only keys with derived values from the closest Pi equivalents.
 * Returns a copy — the original is unmodified.
 */
function normalizePiTheme(colors: Record<string, unknown>): Record<string, unknown> {
  const normalized = { ...colors };

  // Fill Hamr-only keys from Pi equivalents when missing.
  for (const [hamrKey, piFallback] of Object.entries(PI_TO_HAMR_FALLBACKS)) {
    if (!(hamrKey in normalized)) {
      normalized[hamrKey] = colors[piFallback];
    }
  }

  return normalized;
}

// ── JSON Schema Validation (lightweight) ────────────────────────────────────

function validateRequiredKeys(colors: Record<string, unknown>): string[] {
  const missing: string[] = [];
  const required = THEME_COLORS as readonly string[];
  for (const key of required) {
    // Pi-compat keys are optional in Hamr — they exist so Pi themes
    // pass validation and Hamr themes can be used in Pi.
    if (PI_ONLY_COLORS.has(key)) continue;
    if (!(key in colors)) missing.push(key);
  }
  return missing;
}

// ── Theme Class ──────────────────────────────────────────────────────────────

export interface ThemeOptions {
  name?: string;
  sourcePath?: string;
  modelAdaptive?: boolean;
  /** Optional theme-configurable glyphs. */
  glyphs?: ThemeGlyphs;
  /** Optional theme-configurable layout values. */
  layout?: { cardPadX?: number; cardPadY?: number };
}

export interface ThemeGlyphs {
  spinner?: string[];
  [state: string]: string | string[] | undefined;
}

const GLYPH_FALLBACKS: Record<string, string> = {
  ready: '◐',
  working: '◐',
  thinking: '◌',
  cancelled: '✕',
  done: '◆',
  error: '✕',
};

const SPINNER_FALLBACK = ['◴', '◷', '◶', '◵'];

export type ThemeChangeCallback = (theme: HamrTheme) => void;

export class HamrTheme {
  readonly name: string;
  readonly sourcePath?: string;
  readonly modelAdaptive: boolean;
  private fgMap: Map<string, string> = new Map();
  private bgMap: Map<string, string> = new Map();
  readonly mode: ColorMode;
  private watcher?: fs.FSWatcher;
  private onChange?: ThemeChangeCallback;
  private _glyphs: ThemeGlyphs;
  private _layout: { cardPadX: number; cardPadY: number };

  constructor(
    fgColors: Record<string, ResolvedColor>,
    bgColors: Record<string, ResolvedColor>,
    mode: ColorMode,
    options: ThemeOptions = {},
  ) {
    this.name = options.name || 'unknown';
    this.sourcePath = options.sourcePath;
    this.mode = mode;
    this.modelAdaptive = options.modelAdaptive ?? true;
    this._glyphs = options.glyphs || {};
    this._layout = { cardPadX: options.layout?.cardPadX ?? 2, cardPadY: options.layout?.cardPadY ?? 0 };

    for (const [key, value] of Object.entries(fgColors)) {
      this.fgMap.set(key, fgAnsi(value, mode));
    }
    for (const [key, value] of Object.entries(bgColors)) {
      this.bgMap.set(key, bgAnsi(value, mode));
    }
  }

  fg(color: ThemeColor, text: string): string {
    const ansi = this.fgMap.get(color);
    if (!ansi) throw new Error(`Unknown theme foreground color: ${color}`);
    return `${ansi}${text}\x1b[39m`;
  }

  bg(color: ThemeColor, text: string): string {
    const ansi = this.bgMap.get(color);
    if (!ansi) throw new Error(`Unknown theme background color: ${color}`);
    return `${ansi}${text}\x1b[49m`;
  }

  getFgAnsi(color: ThemeColor): string {
    const ansi = this.fgMap.get(color);
    if (!ansi) throw new Error(`Unknown theme foreground color: ${color}`);
    return ansi;
  }

  getBgAnsi(color: ThemeColor): string {
    const ansi = this.bgMap.get(color);
    if (!ansi) throw new Error(`Unknown theme background color: ${color}`);
    return ansi;
  }

  resetFg(): string {
    return '\x1b[39m';
  }

  resetBg(): string {
    return '\x1b[49m';
  }

  bold(text: string): string {
    return `\x1b[1m${text}\x1b[22m`;
  }

  italic(text: string): string {
    return `\x1b[3m${text}\x1b[23m`;
  }

  dim(text: string): string {
    return `\x1b[2m${text}\x1b[22m`;
  }

  watch(callback: ThemeChangeCallback): void {
    if (!this.sourcePath || !fs.existsSync(this.sourcePath)) return;
    this.onChange = callback;
    try {
      this.watcher?.close();
      this.watcher = fs.watch(this.sourcePath, (eventType) => {
        if (eventType === 'change') {
          try {
            const loaded = loadThemeFile(this.sourcePath!);
            const mode = this.mode;
            const newTheme = buildTheme(loaded, mode, { name: loaded.name, sourcePath: this.sourcePath });
            this.onChange?.(newTheme);
          } catch (err) {
            console.error(`Theme hot-reload failed for ${this.sourcePath}:`, err);
          }
        }
      });
    } catch (err) {
      console.error(`Theme watch failed for ${this.sourcePath}:`, err);
    }
  }

  unwatch(): void {
    this.watcher?.close();
    this.watcher = undefined;
    this.onChange = undefined;
  }

  /** Build a SelectListTheme from this theme's color tokens. */
  makeSelectListTheme(): import('sexy-tui-rs').SelectListTheme {
    return {
      selectedPrefix: (text: string) => this.fg('accent', text),
      selectedText: (text: string) => this.fg('text', text),
      description: (text: string) => this.fg('muted', text),
      scrollInfo: (text: string) => this.fg('dim', text),
      noMatch: (text: string) => this.fg('muted', text),
    };
  }

  // ── Theme-configurable glyphs (with hardcoded fallbacks) ──────────────

  /** Spinner frames for the status bar activity indicator. */
  spinnerFrames(): string[] {
    const s = this._glyphs.spinner;
    return Array.isArray(s) && s.length > 0 ? s : SPINNER_FALLBACK;
  }

  /** Activity glyph for a given state. */
  activityGlyph(state: string): string {
    const g = this._glyphs[state];
    return typeof g === 'string' ? g : (GLYPH_FALLBACKS[state] ?? '◐');
  }

  /** Horizontal padding (in chars) for card backgrounds. Theme-configurable. */
  cardPadX(): number {
    return this._layout.cardPadX;
  }

  /** Vertical padding (blank lines above/below each card). Theme-configurable. */
  cardPadY(): number {
    return this._layout.cardPadY;
  }
}

// ── Loading & Building ───────────────────────────────────────────────────────

function loadThemeFile(filePath: string): ThemeJson {
  const raw = fs.readFileSync(filePath, 'utf-8');
  const parsed = JSON.parse(raw);

  if (!parsed.name || !parsed.colors) {
    throw new Error(`Invalid theme file: ${filePath} — missing "name" or "colors"`);
  }

  // Pi cross-compatibility: detect Pi themes (they have Pi-only keys like
  // "thinkingOff" or "bashMode" but may be missing Hamr-only keys like
  // "toolWarningBg" or "editorBg").  Normalize by filling in Hamr-only keys
  // from the closest Pi equivalents before validation.
  const hasPiKeys = Object.keys(parsed.colors).some((k) => PI_ONLY_COLORS.has(k));
  if (hasPiKeys) {
    parsed.colors = normalizePiTheme(parsed.colors);
  }

  const missing = validateRequiredKeys(parsed.colors);
  if (missing.length > 0) {
    throw new Error(`Theme "${parsed.name}" missing required colors: ${missing.join(', ')}`);
  }

  return parsed as ThemeJson;
}

function buildTheme(json: ThemeJson, mode: ColorMode, options: ThemeOptions = {}): HamrTheme {
  const resolved = resolveColors(json.colors, json.vars || {});

  // Fill defaults for Pi-cross-compat keys when the theme doesn't define them.
  // This lets Hamr themes be valid Pi themes without every theme author
  // needing to know about Pi's thinking level border system.
  for (const key of PI_ONLY_COLORS) {
    if (!(key in resolved)) {
      const fallback = PI_DEFAULTS[key as keyof typeof PI_DEFAULTS];
      if (fallback && fallback in resolved) resolved[key] = resolved[fallback];
    }
  }

  const fgColors: Record<string, ResolvedColor> = {};
  const bgColors: Record<string, ResolvedColor> = {};

  const fgSet = new Set(FG_COLORS);
  const bgSet = new Set(BG_COLORS);

  for (const [key, value] of Object.entries(resolved)) {
    if (fgSet.has(key as ThemeColor)) fgColors[key] = value;
    if (bgSet.has(key as ThemeColor)) bgColors[key] = value;
  }

  return new HamrTheme(fgColors, bgColors, mode, {
    name: options.name || json.name,
    sourcePath: options.sourcePath,
    modelAdaptive: json.modelAdaptive,
    glyphs: (json as { glyphs?: ThemeGlyphs }).glyphs,
    layout: json.layout,
  });
}

// ── Public API ───────────────────────────────────────────────────────────────

export interface LoadThemeResult {
  theme: HamrTheme;
  sourcePath: string;
  builtIn: boolean;
}

export function loadTheme(name: string): LoadThemeResult {
  const mode = detectColorMode();
  const builtInDir = getBuiltInDir();
  const userDir = getUserThemesDir();

  const filePath = findThemeFile(name, builtInDir, userDir);
  if (!filePath) {
    // Fall back to default.json
    const fallback = findThemeFile('default', builtInDir, userDir);
    if (!fallback) throw new Error(`No theme found for "${name}" and no default.json fallback available`);
    const json = loadThemeFile(fallback);
    const theme = buildTheme(json, mode, { name: json.name, sourcePath: fallback });
    return { theme, sourcePath: fallback, builtIn: true };
  }

  const json = loadThemeFile(filePath);
  const isBuiltIn = !filePath.startsWith(userDir);
  const theme = buildTheme(json, mode, { name: json.name, sourcePath: filePath });
  return { theme, sourcePath: filePath, builtIn: isBuiltIn };
}

export function loadDefaultTheme(): LoadThemeResult {
  return loadTheme('default');
}

export function listAvailableThemes(): { name: string; builtIn: boolean }[] {
  const builtInDir = getBuiltInDir();
  const userDir = getUserThemesDir();
  return listThemes(builtInDir, userDir).map((t) => ({ name: t.name, builtIn: t.builtIn }));
}

export function formatThemeCost(_theme: HamrTheme): string {
  return _theme.name;
}
