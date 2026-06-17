/**
 * models.json loader — pi-compatible custom provider/model configuration.
 *
 * Loads ~/.hamr/models.json (or HAMR_MODELS_JSON path), validates schema,
 * resolves config values ($ENV, ${ENV}, !command), and merges with
 * built-in provider presets.
 *
 * Format mirrors pi's models.json for familiarity:
 *   ~/.pi/agent/models.json → ~/.hamr/models.json
 */

import { execSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import type { ModelConfig, ProviderConfig, ThinkingLevel } from "./schema.js";
import type { ModelCompatConfig, ThinkingFormat, ThinkingLevelMap } from "./types.js";

// ─── Types ───────────────────────────────────────────

/** Raw models.json shape (before resolution). */
export interface ModelsJsonProvider {
	name?: string;
	baseUrl?: string;
	base_url?: string;
	api?: string;
	apiKey?: string;
	api_key?: string;
	apiKeyEnv?: string;
	api_key_env?: string;
	headers?: Record<string, string>;
	/** Add Authorization: Bearer <apiKey> automatically. */
	authHeader?: boolean;
	auth_header?: boolean;
	compat?: ModelCompatConfig;
	models?: ModelsJsonModel[];
	modelOverrides?: Record<string, ModelsJsonModelOverride>;
	model_overrides?: Record<string, ModelsJsonModelOverride>;
	/** Auto-discover models from the /v1/models endpoint. */
	autoDiscover?: boolean;
	auto_discover?: boolean;
	modelsEndpoint?: string;
	models_endpoint?: string;
}

export interface ModelsJsonModel {
	id: string;
	name?: string;
	api?: string;
	baseUrl?: string;
	base_url?: string;
	reasoning?: boolean;
	thinkingLevelMap?: ThinkingLevelMap;
	thinking_level_map?: ThinkingLevelMap;
	thinkingFormat?: ThinkingFormat;
	thinking_format?: ThinkingFormat;
	toolCallParser?: string;
	tool_call_parser?: string;
	input?: Array<"text" | "image">;
	cost?: { input: number; output: number; cacheRead: number; cacheWrite: number };
	contextWindow?: number;
	context_window?: number;
	maxTokens?: number;
	max_tokens?: number;
	headers?: Record<string, string>;
	compat?: ModelCompatConfig;
	supportsNativeToolCalls?: boolean;
	supports_native_tool_calls?: boolean;
	maxTokensField?: "max_completion_tokens" | "max_tokens";
	max_tokens_field?: "max_completion_tokens" | "max_tokens";
}

export interface ModelsJsonModelOverride {
	name?: string;
	reasoning?: boolean;
	thinkingLevelMap?: ThinkingLevelMap;
	thinking_level_map?: ThinkingLevelMap;
	input?: Array<"text" | "image">;
	cost?: Partial<{ input: number; output: number; cacheRead: number; cacheWrite: number }>;
	contextWindow?: number;
	context_window?: number;
	maxTokens?: number;
	max_tokens?: number;
	headers?: Record<string, string>;
	compat?: ModelCompatConfig;
	toolCallParser?: string;
	tool_call_parser?: string;
}

export interface ModelsJsonConfig {
	providers: Record<string, ModelsJsonProvider>;
}

export interface LoadModelsJsonResult {
	/** Parsed provider configs ready for merging into the Hamr config system. */
	providers: Record<string, ProviderConfig>;
	/** Per-model overrides for built-in models (providerName → modelId → overrides). */
	modelOverrides: Record<string, Record<string, ModelsJsonModelOverride>>;
	/** Error message if loading/parsing failed. */
	error?: string;
}

// ─── Config value resolution ────────────────────────

const COMMAND_TIMEOUT_MS = 10_000;

/**
 * Resolve a config value string that may contain:
 * - $ENV_VAR or ${ENV_VAR} → environment variable
 * - !command → shell command output
 * - $$ → literal $
 * - $! → literal !
 */
export function resolveConfigValue(raw: string): string {
	if (!raw) return raw;

	// Command execution: starts with "!"
	if (raw.startsWith("!")) {
		const cmd = raw.slice(1);
		try {
			const result = execSync(cmd, {
				encoding: "utf-8",
				timeout: COMMAND_TIMEOUT_MS,
				stdio: ["ignore", "pipe", "pipe"],
			});
			return result.trim();
		} catch (err) {
			const msg = err instanceof Error ? err.message : String(err);
			throw new Error(`Failed to execute config command "${cmd}": ${msg}`);
		}
	}

	// Escape sequences
	let result = raw.replace(/\$\$/g, "\x00DOLLAR\x00"); // placeholder
	result = result.replace(/\$!/g, "\x00BANG\x00");

	// ${ENV_VAR} form
	result = result.replace(/\$\{(\w+)\}/g, (_m, name: string) => {
		const val = process.env[name];
		if (val === undefined) throw new Error(`Environment variable "${name}" is not set`);
		return val;
	});

	// $ENV_VAR form (matches $WORD where WORD is uppercase/lowercase alphanumeric + underscore)
	result = result.replace(/\$(\w+)/g, (_m, name: string) => {
		// Only treat as env var if it's ALL_UPPER_CASE or camelCase with at least one uppercase
		const val = process.env[name];
		if (val === undefined) {
			// If it looks like a typical env var name, throw; otherwise treat as literal
			if (/[A-Z]/.test(name)) throw new Error(`Environment variable "${name}" is not set`);
			return `$${name}`; // literal (e.g. $someLowercase)
		}
		return val;
	});

	result = result.replace(/\x00DOLLAR\x00/g, "$");
	result = result.replace(/\x00BANG\x00/g, "!");

	return result;
}

/**
 * Resolve headers object, expanding values.
 */
function resolveHeaders(headers?: Record<string, string>): Record<string, string> {
	if (!headers || Object.keys(headers).length === 0) return {};
	const resolved: Record<string, string> = {};
	for (const [key, value] of Object.entries(headers)) {
		resolved[key] = resolveConfigValue(value);
	}
	return resolved;
}

// ─── Compat merging ─────────────────────────────────

/** Deep merge model-level compat with provider-level defaults. */
function mergeCompat(
	providerCompat: ModelCompatConfig | undefined,
	modelCompat: ModelCompatConfig | undefined,
): ModelCompatConfig | undefined {
	if (!providerCompat && !modelCompat) return undefined;
	const merged: ModelCompatConfig = {};

	// OpenAI compat
	if (providerCompat?.openai || modelCompat?.openai) {
		merged.openai = { ...providerCompat?.openai, ...modelCompat?.openai };
	}

	// Anthropic compat
	if (providerCompat?.anthropic || modelCompat?.anthropic) {
		merged.anthropic = { ...providerCompat?.anthropic, ...modelCompat?.anthropic };
	}

	return merged;
}

// ─── Normalize camelCase/snake_case ─────────────────

function normalizeBool(val: boolean | undefined, alt: boolean | undefined): boolean | undefined {
	return val ?? alt;
}

function normalizeString(val: string | undefined, alt: string | undefined): string | undefined {
	return val ?? alt;
}

// ─── Loading ────────────────────────────────────────

/** Default path for models.json. Respects HAMR_MODELS_JSON env var. */
export function getModelsJsonPath(): string {
	const envPath = process.env.HAMR_MODELS_JSON;
	if (envPath) return envPath;
	return join(homedir(), ".hamr", "models.json");
}

/**
 * Load and parse models.json into ProviderConfig entries.
 * Returns an error string on failure (invalid JSON, schema issues, etc.)
 * along with any successfully parsed entries.
 */
export function loadModelsJson(modelsJsonPath?: string): LoadModelsJsonResult {
	const path = modelsJsonPath ?? getModelsJsonPath();

	if (!existsSync(path)) {
		return { providers: {}, modelOverrides: {}, error: undefined };
	}

	let raw: unknown;
	try {
		const content = readFileSync(path, "utf-8");
		raw = JSON.parse(stripJsonComments(content));
	} catch (err) {
		const msg = err instanceof Error ? err.message : String(err);
		return { providers: {}, modelOverrides: {}, error: `Failed to parse ${path}: ${msg}` };
	}

	const config = raw as ModelsJsonConfig;
	if (!config.providers || typeof config.providers !== "object") {
		return { providers: {}, modelOverrides: {}, error: `${path} is missing "providers" section` };
	}

	const providers: Record<string, ProviderConfig> = {};
	const modelOverrides: Record<string, Record<string, ModelsJsonModelOverride>> = {};

	for (const [providerId, rawProvider] of Object.entries(config.providers)) {
		try {
			const provider = parseProvider(providerId, rawProvider);
			providers[providerId] = provider;

			// Collect model overrides
			const overrides = rawProvider.modelOverrides ?? rawProvider.model_overrides;
			if (overrides && Object.keys(overrides).length > 0) {
				modelOverrides[providerId] = { ...overrides };
			}
		} catch (err) {
			// Don't let one bad provider fail the whole load.
			const msg = err instanceof Error ? err.message : String(err);
			console.warn(`[hamr] models.json: skipping provider "${providerId}": ${msg}`);
		}
	}

	return { providers, modelOverrides, error: undefined };
}

/** Strip JSON comments (// and /*) before parsing. */
function stripJsonComments(json: string): string {
	// Remove single-line comments
	let result = json.replace(/\/\/.*$/gm, "");
	// Remove multi-line comments
	result = result.replace(/\/\*[\s\S]*?\*\//g, "");
	return result;
}

/**
 * Parse a single provider from models.json into a hamr ProviderConfig.
 */
function parseProvider(providerId: string, raw: ModelsJsonProvider): ProviderConfig {
	const compat = resolveProviderCompat(raw);
	const models = parseProviderModels(providerId, raw, compat);
	const overrides: Record<string, Partial<ModelConfig>> | undefined = parseModelOverrides(raw);

	return {
		id: providerId,
		name: raw.name,
		compatibility: resolveApiToCompatibility(raw.api),
		enabled: true,
		baseUrl: raw.baseUrl ?? raw.base_url,
		apiKey: raw.apiKey ?? raw.api_key,
		apiKeyEnv: raw.apiKeyEnv ?? raw.api_key_env,
		headers: resolveHeaders(raw.headers),
		models,
		compat,
		autoDiscover: normalizeBool(raw.autoDiscover, raw.auto_discover),
		modelsEndpoint: normalizeString(raw.modelsEndpoint, raw.models_endpoint),
		modelOverrides: overrides,
	};
}

/** Resolve API name to hamr compatibility value. */
function resolveApiToCompatibility(api?: string): "openai-compatible" | "anthropic-compatible" {
	if (!api) return "openai-compatible";
	switch (api.toLowerCase()) {
		case "anthropic-messages":
		case "anthropic":
			return "anthropic-compatible";
		default:
			return "openai-compatible";
	}
}

/** Build provider-level compat from models.json provider entry. */
function resolveProviderCompat(raw: ModelsJsonProvider): ModelCompatConfig | undefined {
	if (!raw.compat) return undefined;
	return raw.compat;
}

/** Parse models array from a provider. */
function parseProviderModels(
	_providerId: string,
	raw: ModelsJsonProvider,
	providerCompat: ModelCompatConfig | undefined,
): ModelConfig[] {
	const modelDefs = raw.models ?? [];
	if (modelDefs.length === 0) return [];

	return modelDefs.map((m) => {
		const thinkingLevels: ThinkingLevel[] | undefined = m.reasoning
			? ["off", "low", "medium", "high", "xhigh"]
			: undefined;

		const modelCompatFields = mergeCompat(providerCompat, m.compat);

		return {
			id: m.id,
			displayName: m.name,
			contextWindow: m.contextWindow ?? m.context_window,
			supportsThinking: m.reasoning,
			thinkingLevels,
			maxOutputTokens: m.maxTokens ?? m.max_tokens,
			supportsVision: (m.input ?? ["text"]).includes("image"),
			// Per-model compat fields
			toolCallParser: m.toolCallParser ?? m.tool_call_parser,
			thinkingFormat: m.thinkingFormat ?? m.thinking_format,
			thinkingLevelMap: m.thinkingLevelMap ?? m.thinking_level_map,
			compat: modelCompatFields,
			supportsNativeToolCalls: normalizeBool(m.supportsNativeToolCalls, m.supports_native_tool_calls),
			maxTokensField: normalizeString(m.maxTokensField, m.max_tokens_field) as
				| "max_completion_tokens"
				| "max_tokens"
				| undefined,
		};
	});
}

/** Parse per-model overrides for built-in models. */
function parseModelOverrides(raw: ModelsJsonProvider): Record<string, Partial<ModelConfig>> | undefined {
	const overrides = raw.modelOverrides ?? raw.model_overrides;
	if (!overrides || Object.keys(overrides).length === 0) return undefined;

	const result: Record<string, Partial<ModelConfig>> = {};
	for (const [modelId, override] of Object.entries(overrides)) {
		result[modelId] = {
			id: modelId,
			displayName: override.name,
			contextWindow: override.contextWindow ?? override.context_window,
			supportsThinking: override.reasoning,
			supportsVision: override.input?.includes("image"),
			maxOutputTokens: override.maxTokens ?? override.max_tokens,
			toolCallParser: override.toolCallParser ?? override.tool_call_parser,
			thinkingLevelMap: override.thinkingLevelMap ?? override.thinking_level_map,
			compat: override.compat,
		};
	}
	return result;
}
