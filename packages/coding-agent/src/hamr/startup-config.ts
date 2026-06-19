import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { ThinkingLevel } from "@hamr/agent";
import type { Model } from "@hamr/ai";
import { parse as parseToml } from "toml";
import type { ProviderConfig } from "../core/extensions/types.ts";
import type { ModelRegistry } from "../core/model-registry.ts";
import { detectParserId } from "./providers/parsers/types.ts";
import { discoverRelayModels as discoverRelayEndpointModels } from "./providers/relay-provider.ts";

type HamrThinkingLevel = ThinkingLevel | "auto";

interface HamrModelConfig {
	id: string;
	display_name?: string;
	displayName?: string;
	context_window?: number;
	contextWindow?: number;
	max_output_tokens?: number;
	maxOutputTokens?: number;
	supports_thinking?: boolean;
	supportsThinking?: boolean;
	supports_vision?: boolean;
	supportsVision?: boolean;
	thinking_levels?: HamrThinkingLevel[];
	thinkingLevels?: HamrThinkingLevel[];
	default_thinking?: HamrThinkingLevel;
	defaultThinking?: HamrThinkingLevel;
	tool_call_parser?: string;
	toolCallParser?: string;
}

interface HamrProviderConfig {
	enabled?: boolean;
	name?: string;
	compatibility?: "openai-compatible" | "anthropic-compatible";
	base_url?: string;
	baseUrl?: string;
	api_key?: string;
	apiKey?: string;
	api_key_env?: string;
	apiKeyEnv?: string;
	headers?: Record<string, string>;
	custom_headers?: Record<string, string>;
	customHeaders?: Record<string, string>;
	models?: HamrModelConfig[];
	tool_call_parser?: string;
	toolCallParser?: string;
	/**
	 * Whether this provider is a cloud provider. Cloud models may dispatch
	 * parallel subagents; relay/local providers may not. Defaults to false for
	 * configured providers (which are local/relay by convention).
	 */
	cloud?: boolean;
}

export interface HamrStartupConfig {
	active?: {
		provider?: string;
		model?: string;
		thinking?: HamrThinkingLevel;
	};
	providers: Record<string, HamrProviderConfig>;
	sourcePaths: string[];
}

export interface HamrProviderRegistration {
	name: string;
	config: ProviderConfig;
	parserByModel: Map<string, string>;
}

const DEFAULT_RELAY_BASE_URL = "http://127.0.0.1:1234/v1";
const LOCAL_API_KEY = "not-needed";
/** Env override for the relay endpoint base URL (matches synax's HAMR_ prefix convention). */
const RELAY_BASE_URL_ENV = "HAMR_RELAY_BASE_URL";
/** When set to "1", skip network model discovery (mirrors synax's test gate). */
const SKIP_NETWORK_ENV = "HAMR_TEST_SKIP_NETWORK";

/**
 * Expand ${VAR} and $VAR references in a config string from process.env,
 * matching synax's config interpolation. Unset vars expand to "".
 */
function expandEnv(value: string | undefined): string | undefined {
	if (value === undefined) return undefined;
	return value
		.replace(/\$\{(\w+)\}/g, (_, name: string) => process.env[name] ?? "")
		.replace(/\$(\w+)/g, (_, name: string) => process.env[name] ?? "");
}

/** Resolve the effective relay base URL: env override > config > default. */
function resolveRelayBaseUrl(provider: HamrProviderConfig): string {
	const fromEnv = process.env[RELAY_BASE_URL_ENV]?.trim();
	if (fromEnv) return fromEnv;
	return providerBaseUrl(provider) ?? DEFAULT_RELAY_BASE_URL;
}

function globalHamrConfigPath(): string {
	const home = process.env.HOME || process.env.USERPROFILE;
	return home ? join(home, ".config", "hamr", "config.toml") : "";
}

function discoverLocalConfigPath(baseDir: string): string | undefined {
	const candidate = join(baseDir, ".hamr.toml");
	if (existsSync(candidate)) return candidate;
	const parent = dirname(baseDir);
	if (parent === baseDir) return undefined;
	return discoverLocalConfigPath(parent);
}

function parseConfigFile(path: string): Partial<HamrStartupConfig> | undefined {
	if (!path || !existsSync(path)) return undefined;
	const parsed = parseToml(readFileSync(path, "utf8")) as Record<string, unknown>;
	return {
		active:
			typeof parsed.active === "object" && parsed.active ? (parsed.active as HamrStartupConfig["active"]) : undefined,
		providers:
			typeof parsed.providers === "object" && parsed.providers
				? (parsed.providers as Record<string, HamrProviderConfig>)
				: undefined,
	};
}

function mergeConfig(base: HamrStartupConfig, next: Partial<HamrStartupConfig>, sourcePath: string): HamrStartupConfig {
	return {
		active: { ...(base.active ?? {}), ...(next.active ?? {}) },
		providers: {
			...base.providers,
			...(next.providers ?? {}),
		},
		sourcePaths: [...base.sourcePaths, sourcePath],
	};
}

export function loadHamrStartupConfig(cwd: string = process.cwd()): HamrStartupConfig {
	let config: HamrStartupConfig = {
		active: {
			provider: "relay",
			thinking: "off",
		},
		providers: {
			relay: {
				enabled: true,
				name: "Relay",
				compatibility: "openai-compatible",
				base_url: DEFAULT_RELAY_BASE_URL,
			},
		},
		sourcePaths: [],
	};

	const globalPath = globalHamrConfigPath();
	const globalConfig = parseConfigFile(globalPath);
	if (globalConfig) config = mergeConfig(config, globalConfig, globalPath);

	const localPath = discoverLocalConfigPath(cwd);
	const localConfig = localPath ? parseConfigFile(localPath) : undefined;
	if (localPath && localConfig) config = mergeConfig(config, localConfig, localPath);

	return config;
}

function envReference(name?: string): string | undefined {
	if (!name) return undefined;
	return `$${name}`;
}

function providerBaseUrl(provider: HamrProviderConfig): string | undefined {
	return provider.base_url ?? provider.baseUrl;
}

function providerHeaders(provider: HamrProviderConfig): Record<string, string> | undefined {
	const headers = provider.headers ?? provider.custom_headers ?? provider.customHeaders;
	if (!headers || Object.keys(headers).length === 0) return undefined;
	// Expand ${VAR}/$VAR references so custom auth headers work for discovery.
	const expanded: Record<string, string> = {};
	for (const [key, value] of Object.entries(headers)) {
		expanded[key] = expandEnv(value) ?? value;
	}
	return expanded;
}

/**
 * Resolve a concrete API key for talking to the endpoint (model discovery).
 * Unlike the registration apiKey (which pi resolves itself), this returns the
 * actual value: a literal, env interpolation, or the named env var. Local
 * relays usually need no key, so this may legitimately be undefined.
 */
function resolveProviderApiKey(provider: HamrProviderConfig): string | undefined {
	const literal = expandEnv(provider.api_key ?? provider.apiKey)?.trim();
	if (literal) return literal;
	const envName = provider.api_key_env ?? provider.apiKeyEnv;
	const fromEnv = envName ? process.env[envName]?.trim() : undefined;
	return fromEnv || undefined;
}

function normalizeThinking(level: HamrThinkingLevel | undefined): ThinkingLevel | undefined {
	if (!level || level === "auto") return undefined;
	return level;
}

/**
 * Auto-detect models served by an OpenAI-compatible relay endpoint and map
 * them into Hamr's model-config shape. The endpoint complexity is masked: the
 * user just sees whatever models the relay is currently serving. Returns an
 * empty list when the endpoint is unreachable (no crash, falls back to config).
 */
async function discoverRelayModels(
	baseUrl: string,
	apiKey: string | undefined,
	headers: Record<string, string> | undefined,
): Promise<HamrModelConfig[]> {
	if (process.env[SKIP_NETWORK_ENV] === "1") return [];
	const discovered = await discoverRelayEndpointModels(baseUrl, apiKey, headers);
	return discovered.map((model) => ({
		id: model.id,
		display_name: model.displayName,
		context_window: model.contextWindow,
		max_output_tokens: model.maxOutputTokens,
		supports_thinking: model.supportsThinking,
		thinking_levels: model.thinkingLevels as HamrThinkingLevel[],
		// Pass supportsVision through unmodified so that modelToProviderModel's
		// `?? true` default applies consistently: undefined → vision-capable.
		// Relays that explicitly advertise "text-only" get supportsVision=false.
		supports_vision: model.supportsVision,
	}));
}

function modelContextWindow(model: HamrModelConfig): number | undefined {
	return model.context_window ?? model.contextWindow;
}

function modelMaxOutputTokens(model: HamrModelConfig): number | undefined {
	return model.max_output_tokens ?? model.maxOutputTokens;
}

function modelSupportsVision(model: HamrModelConfig): boolean | undefined {
	return model.supports_vision ?? model.supportsVision;
}

function mergeDiscoveredModel(configured: HamrModelConfig, discovered: HamrModelConfig): HamrModelConfig {
	return {
		...discovered,
		...configured,
		display_name:
			configured.display_name ?? configured.displayName ?? discovered.display_name ?? discovered.displayName,
		context_window: modelContextWindow(discovered) ?? modelContextWindow(configured),
		max_output_tokens: modelMaxOutputTokens(discovered) ?? modelMaxOutputTokens(configured),
		supports_vision: modelSupportsVision(configured) ?? modelSupportsVision(discovered),
		supports_thinking:
			configured.supports_thinking ??
			configured.supportsThinking ??
			discovered.supports_thinking ??
			discovered.supportsThinking,
		thinking_levels:
			configured.thinking_levels ??
			configured.thinkingLevels ??
			discovered.thinking_levels ??
			discovered.thinkingLevels,
		default_thinking: configured.default_thinking ?? configured.defaultThinking ?? discovered.default_thinking,
		tool_call_parser:
			configured.tool_call_parser ??
			configured.toolCallParser ??
			discovered.tool_call_parser ??
			discovered.toolCallParser,
	};
}

function mergeProviderModels(configured: HamrModelConfig[], discovered: HamrModelConfig[]): HamrModelConfig[] {
	if (configured.length === 0) return discovered;
	if (discovered.length === 0) return configured;

	const discoveredById = new Map(discovered.map((model) => [model.id.toLowerCase(), model]));
	const configuredIds = new Set(configured.map((model) => model.id.toLowerCase()));
	return [
		...configured.map((model) => {
			const discoveredModel = discoveredById.get(model.id.toLowerCase());
			return discoveredModel ? mergeDiscoveredModel(model, discoveredModel) : model;
		}),
		...discovered.filter((model) => !configuredIds.has(model.id.toLowerCase())),
	];
}

/**
 * Resolve the model list for a provider. Explicit config models define local
 * intent, but OpenAI-compatible providers are still probed so live endpoint
 * facts (especially context length) can override stale or incomplete config.
 */
async function resolveProviderModels(providerId: string, provider: HamrProviderConfig): Promise<HamrModelConfig[]> {
	const configured = provider.models ?? [];
	const compatibility = provider.compatibility ?? "openai-compatible";
	if (compatibility !== "openai-compatible") return configured;
	const baseUrl = providerId === "relay" ? resolveRelayBaseUrl(provider) : providerBaseUrl(provider);
	if (!baseUrl) return configured;
	const discovered = await discoverRelayModels(baseUrl, resolveProviderApiKey(provider), providerHeaders(provider));
	return mergeProviderModels(configured, discovered);
}

function detectThinkingFormat(modelId: string): string | undefined {
	const lower = modelId.toLowerCase();
	if (/deepseek-?r1/.test(lower)) return "deepseek";
	if (/\bqwen3\b/.test(lower) || /\bqwen3[.-]/.test(lower)) return "qwen";
	return undefined;
}

function modelToProviderModel(
	model: HamrModelConfig,
	provider: HamrProviderConfig,
): NonNullable<ProviderConfig["models"]>[number] {
	const thinking = model.supports_thinking ?? model.supportsThinking ?? false;
	const supportsVision = model.supports_vision ?? model.supportsVision ?? true;
	const contextWindow = modelContextWindow(model) ?? 0;
	const compatibility = provider.compatibility ?? "openai-compatible";
	const thinkingFormat = detectThinkingFormat(model.id);
	// Derive thinkingLevelMap from the actual levels advertised by the relay.
	//
	// Semantics of thinkingLevelMap (see packages/ai/src/types.ts): a *missing*
	// key means the level is supported and uses the provider default; an explicit
	// `null` marks the level as unsupported. So we translate the relay's flat list
	// of supported levels into "leave advertised levels missing, null out the rest"
	// rather than the other way around.
	const CANONICAL_LEVELS = ["off", "minimal", "low", "medium", "high", "xhigh"] as const;
	const rawLevels = model.thinking_levels ?? (thinking ? ["off", "on"] : []);
	// Relay vocabulary uses "on" (no "minimal"); pi uses "minimal" (no "on").
	// "off" is always offered when thinking is supported, and a bare "on"
	// (supports_thinking with no explicit levels) is treated as max thinking.
	const advertised = new Set<string>(thinking ? ["off"] : []);
	for (const level of rawLevels) {
		advertised.add(level === "on" ? "high" : level);
	}
	const thinkingLevelMap: Record<string, string | null> = {};
	for (const level of CANONICAL_LEVELS) {
		if (!advertised.has(level)) {
			thinkingLevelMap[level] = null; // unsupported
		} else if (level === "xhigh") {
			// xhigh is excluded unless it maps to a defined value.
			thinkingLevelMap.xhigh = "xhigh";
		}
		// Advertised non-xhigh levels are left missing → supported, provider default.
	}
	return {
		id: model.id,
		name: model.display_name ?? model.displayName ?? model.id,
		reasoning: thinking,
		thinkingLevelMap,
		input: supportsVision ? ["text", "image"] : ["text"],
		cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
		contextWindow,
		maxTokens: modelMaxOutputTokens(model) ?? (contextWindow > 0 ? Math.min(16384, contextWindow) : 16384),
		compat:
			compatibility === "openai-compatible"
				? {
						supportsDeveloperRole: false,
						supportsUsageInStreaming: false,
						supportsStrictMode: false,
						maxTokensField: "max_tokens",
						...(thinkingFormat !== undefined ? { thinkingFormat } : {}),
					}
				: undefined,
	};
}

/**
 * Whether the given provider id should be treated as a cloud provider.
 *
 * Configured providers (relay, local LM endpoints) are local by convention and
 * default to non-cloud unless they set `cloud: true`. Providers that are not in
 * the hamr config at all are the built-in cloud providers (anthropic, openai,
 * …), so they are treated as cloud. Used to gate parallel subagent dispatch.
 */
export function isCloudProvider(config: HamrStartupConfig, providerId: string): boolean {
	const provider = config.providers[providerId];
	if (!provider) return true;
	return provider.cloud === true;
}

export async function buildHamrProviderRegistrations(config: HamrStartupConfig): Promise<HamrProviderRegistration[]> {
	const registrations: HamrProviderRegistration[] = [];
	for (const [providerId, provider] of Object.entries(config.providers)) {
		if (provider.enabled === false) continue;
		const compatibility = provider.compatibility ?? "openai-compatible";
		// The relay's base URL falls back to the env override / default; other
		// providers must declare one explicitly.
		const resolvedBaseUrl = providerId === "relay" ? resolveRelayBaseUrl(provider) : providerBaseUrl(provider);
		const baseUrl = expandEnv(resolvedBaseUrl);
		if (!baseUrl) continue;
		const models = await resolveProviderModels(providerId, provider);
		if (models.length === 0) continue;

		// Per-model tool-call parser: explicit config override wins, otherwise
		// auto-detect from the model id (same conservative matching synax uses).
		// This is baked into the registration so the model picks up the right
		// local-model parser the moment it is discovered.
		const parserByModel = new Map<string, string>();
		for (const model of models) {
			const explicit =
				model.tool_call_parser ?? model.toolCallParser ?? provider.tool_call_parser ?? provider.toolCallParser;
			const parser = explicit ?? detectParserId(model.id);
			if (parser) parserByModel.set(model.id, parser);
		}

		registrations.push({
			name: providerId,
			parserByModel,
			config: {
				name: provider.name ?? providerId,
				baseUrl,
				api: compatibility === "anthropic-compatible" ? "anthropic-messages" : "openai-completions",
				apiKey:
					provider.api_key ??
					provider.apiKey ??
					envReference(provider.api_key_env ?? provider.apiKeyEnv) ??
					LOCAL_API_KEY,
				authHeader: false,
				headers: providerHeaders(provider),
				models: models.map((model) => modelToProviderModel(model, provider)),
			},
		});
	}
	return registrations;
}

export function getHamrDefaultModel(
	config: HamrStartupConfig,
	modelRegistry: ModelRegistry,
): { model: Model<any>; thinkingLevel?: ThinkingLevel } | undefined {
	const provider = config.active?.provider ?? "relay";
	const modelId = config.active?.model;
	const model = modelId
		? modelRegistry.find(provider, modelId)
		: modelRegistry.getAll().find((candidate) => candidate.provider === provider);
	if (!model || !modelRegistry.hasConfiguredAuth(model)) return undefined;
	return { model, thinkingLevel: normalizeThinking(config.active?.thinking) };
}
