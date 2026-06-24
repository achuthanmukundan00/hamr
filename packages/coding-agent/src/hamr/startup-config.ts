import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { ThinkingLevel } from "@hamr/agent";
import { type Api, getModel, type Model } from "@hamr/ai";
import { parse as parseToml } from "toml";
import { getAgentDir } from "../config.ts";
import { AuthStorage } from "../core/auth-storage.ts";
import type { ProviderConfig } from "../core/extensions/types.ts";
import type { ModelRegistry } from "../core/model-registry.ts";
import { resolveConfigValueOrThrow, resolveHeadersOrThrow } from "../core/resolve-config-value.ts";
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
	cost?: { input: number; output: number; cacheRead: number; cacheWrite: number };
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

/** Read a provider's `baseUrl` from the agent layer (`models.json`), if present. */
function agentLayerBaseUrl(providerId: string): string | undefined {
	try {
		const modelsPath = join(getAgentDir(), "models.json");
		if (!existsSync(modelsPath)) return undefined;
		const data = JSON.parse(readFileSync(modelsPath, "utf-8")) as {
			providers?: Record<string, { baseUrl?: string }>;
		};
		const baseUrl = data.providers?.[providerId]?.baseUrl;
		return typeof baseUrl === "string" && baseUrl.trim() ? baseUrl.trim() : undefined;
	} catch {
		return undefined;
	}
}

/**
 * Resolve the effective relay base URL, in priority order: env override →
 * hamr.toml `base_url` (if the user pinned one) → agent-layer `models.json`
 * baseUrl (the relay endpoint, same source as its creds) → local default.
 *
 * The agent-layer fallback lets the relay endpoint live entirely in the agent
 * layer, so hamr.toml needs no provider block — it stays a preferences/theming
 * file.
 */
function resolveRelayBaseUrl(provider: HamrProviderConfig): string {
	const fromEnv = process.env[RELAY_BASE_URL_ENV]?.trim();
	if (fromEnv) return fromEnv;
	return providerBaseUrl(provider) ?? agentLayerBaseUrl("relay") ?? DEFAULT_RELAY_BASE_URL;
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
			// The relay endpoint is not pinned here: resolveRelayBaseUrl sources it
			// from the agent layer (models.json) and falls back to the local default,
			// so the relay is determined by the standard discovery path, not config.
			relay: {
				enabled: true,
				name: "Relay",
				compatibility: "openai-compatible",
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

/** Credentials resolved from the agent layer for a provider. */
interface AgentLayerAuth {
	/** Resolved API key (literal value), e.g. the relay bearer token. */
	apiKey?: string;
	/** Custom request headers (e.g. CF-Access-*), resolved from `auth.json` env. */
	headers?: Record<string, string>;
	/** Whether the provider expects an `Authorization: Bearer <apiKey>` header. */
	authHeader: boolean;
}

/**
 * Resolve a provider's credentials from the agent layer (`models.json` +
 * `auth.json`) — the same source the chat path uses via
 * `ModelRegistry.getApiKeyAndHeaders`.
 *
 * Both halves of provider auth historically ignored this: discovery read only
 * the hamr.toml provider block, and the request-time registration apiKey fell
 * back to `LOCAL_API_KEY` ("not-needed"). So a relay-style provider that keeps
 * its real credentials in the agent layer (e.g. behind Cloudflare Access:
 * CF-Access headers in `models.json`, secrets in `auth.json`'s provider `env`)
 * was probed unauthenticated AND registered with a junk key. Resolving here and
 * threading the result into both paths keeps hamr authenticated wherever chat
 * is. Returns `undefined` when the provider has no agent-layer entry (e.g. a
 * plain LM Studio endpoint), so callers fall back to the hamr.toml creds and the
 * unauthenticated probe still works as before.
 *
 * `headers` deliberately excludes `Authorization` — the bearer is carried via
 * `apiKey` + `authHeader` so each consumer (discovery probe vs. registry) adds
 * it the way it expects.
 */
async function resolveAgentLayerAuth(providerId: string): Promise<AgentLayerAuth | undefined> {
	try {
		const modelsPath = join(getAgentDir(), "models.json");
		if (!existsSync(modelsPath)) return undefined;
		const data = JSON.parse(readFileSync(modelsPath, "utf-8")) as {
			providers?: Record<string, { apiKey?: string; headers?: Record<string, string>; authHeader?: boolean }>;
		};
		const providerConfig = data.providers?.[providerId];
		if (!providerConfig) return undefined;

		const authStorage = AuthStorage.create();
		const providerEnv = authStorage.getProviderEnv(providerId);
		const apiKey =
			(await authStorage.getApiKey(providerId, { includeFallback: false })) ??
			(providerConfig.apiKey
				? resolveConfigValueOrThrow(providerConfig.apiKey, `API key for provider "${providerId}"`, providerEnv)
				: undefined);

		const headers = resolveHeadersOrThrow(providerConfig.headers, `provider "${providerId}"`, providerEnv);
		return { apiKey, headers, authHeader: providerConfig.authHeader === true };
	} catch {
		// Any resolution failure (missing env var, unreadable file) falls back to
		// hamr.toml creds — never block startup discovery.
		return undefined;
	}
}

/**
 * Resolve the model list for a provider. Explicit config models define local
 * intent, but OpenAI-compatible providers are still probed so live endpoint
 * facts (especially context length) can override stale or incomplete config.
 *
 * `agentAuth` is resolved once by the caller and reused for both the probe and
 * the registration so they always send the same credentials.
 */
async function resolveProviderModels(
	providerId: string,
	provider: HamrProviderConfig,
	agentAuth: AgentLayerAuth | undefined,
): Promise<HamrModelConfig[]> {
	const configured = provider.models ?? [];
	const compatibility = provider.compatibility ?? "openai-compatible";
	if (compatibility !== "openai-compatible") return configured;
	const baseUrl = providerId === "relay" ? resolveRelayBaseUrl(provider) : providerBaseUrl(provider);
	if (!baseUrl) return configured;
	// Prefer agent-layer creds (models.json/auth.json), exactly as chat does;
	// fall back to the hamr.toml provider block for endpoints configured purely
	// there (e.g. a local LM Studio that needs no auth). discoverRelayModels adds
	// `Authorization: Bearer <apiKey>` itself, so headers stays CF-Access-only.
	const apiKey = agentAuth?.apiKey ?? resolveProviderApiKey(provider);
	const headers = agentAuth?.headers ?? providerHeaders(provider);
	const discovered = await discoverRelayModels(baseUrl, apiKey, headers);
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
	providerId: string,
): NonNullable<ProviderConfig["models"]>[number] {
	const supportsVision = model.supports_vision ?? model.supportsVision ?? true;
	// When this configured entry shadows a known built-in model (same provider + id),
	// inherit pricing and context limits from the built-in unless the config sets them
	// explicitly. This mirrors pi's modelOverride semantics (config merged onto the
	// built-in model), so e.g. adding an API key for the built-in `deepseek` provider
	// keeps its real cost data instead of zeroing it.
	const builtin = getModel(providerId as never, model.id as never) as Model<Api> | undefined;
	// Capability (reasoning + thinking levels) is owned by the model registry for
	// known models. A config entry may DEFINE capability for models the registry
	// doesn't know, and may WIDEN (enable) thinking, but `supports_thinking` leaking
	// to `false` (e.g. a /v1/models probe that omits the field) must never silently
	// strip thinking from a built-in that reasons. So thinking = registry truth OR an
	// explicit config opt-in; an explicit/absent config `false` cannot override the
	// registry. This is the wall pi keeps between preferences and capability.
	const configThinking = model.supports_thinking ?? model.supportsThinking;
	const thinking = (builtin?.reasoning ?? false) || (configThinking ?? false);
	const contextWindow = modelContextWindow(model) ?? builtin?.contextWindow ?? 0;
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
	const derivedThinkingLevelMap: Record<string, string | null> = {};
	for (const level of CANONICAL_LEVELS) {
		if (!advertised.has(level)) {
			derivedThinkingLevelMap[level] = null; // unsupported
		} else if (level === "xhigh") {
			// xhigh is excluded unless it maps to a defined value.
			derivedThinkingLevelMap.xhigh = "xhigh";
		}
		// Advertised non-xhigh levels are left missing → supported, provider default.
	}
	// For a built-in reasoning model the registry's thinkingLevelMap is authoritative,
	// unless the config explicitly advertises its own level list (to widen/customize).
	// Otherwise we'd narrow the levels (e.g. force deepseek-v4 down to off/on) instead
	// of honoring the registry's high/xhigh → "high"/"max" mapping.
	const hasExplicitLevels = Array.isArray(model.thinking_levels);
	const thinkingLevelMap =
		builtin?.reasoning && !hasExplicitLevels ? builtin.thinkingLevelMap : derivedThinkingLevelMap;
	return {
		id: model.id,
		name: model.display_name ?? model.displayName ?? model.id,
		reasoning: thinking,
		thinkingLevelMap,
		input: supportsVision ? ["text", "image"] : ["text"],
		cost: model.cost ?? builtin?.cost ?? { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
		contextWindow,
		maxTokens:
			modelMaxOutputTokens(model) ??
			builtin?.maxTokens ??
			(contextWindow > 0 ? Math.min(thinking ? 65536 : 16384, contextWindow) : thinking ? 65536 : 16384),
		compat:
			compatibility === "openai-compatible"
				? {
						supportsDeveloperRole: false,
						// Request stream_options.include_usage. llama.cpp (build 9634+) and the
						// relay both honor it and emit a final usage-only chunk, which is what
						// makes real token counts — and thus the context-window % — work. The
						// WIP-baseline default was a conservative `false`, which suppressed usage
						// and forced the footer onto a chars/4 estimate (showing 0%). Opt in.
						supportsUsageInStreaming: true,
						supportsStrictMode: false,
						maxTokensField: "max_tokens",
						// Enable Anthropic-style cache_control markers on system prompt,
						// tools, and last user message so relay backends that support
						// prompt caching (vLLM, LiteLLM) can serve from cache. Markers
						// are harmless for backends that don't support caching.
						cacheControlFormat: "anthropic",
						// Route requests to the same replica for cache affinity.
						sendSessionAffinityHeaders: true,
						// Relay backends (LiteLLM, vLLM) typically support long cache
						// retention with Anthropic-format ttl markers.
						supportsLongCacheRetention: true,
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
		// Resolve agent-layer creds once and reuse for both the discovery probe
		// and the request-time registration, so they never disagree.
		const agentAuth = await resolveAgentLayerAuth(providerId);
		const models = await resolveProviderModels(providerId, provider, agentAuth);
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
				// Request-time creds, in priority order: agent-layer (models.json /
				// auth.json, same as chat) → hamr.toml literal → hamr.toml env ref →
				// "not-needed" for keyless local endpoints (LM Studio). Previously this
				// only saw the hamr.toml chain, so a relay whose key lives in the agent
				// layer was registered with "not-needed" and every request 401'd.
				apiKey:
					agentAuth?.apiKey ??
					provider.api_key ??
					provider.apiKey ??
					envReference(provider.api_key_env ?? provider.apiKeyEnv) ??
					LOCAL_API_KEY,
				// When the agent layer says this provider wants a bearer, let the
				// registry attach `Authorization: Bearer <apiKey>` at request time.
				authHeader: agentAuth?.authHeader ?? false,
				headers: agentAuth?.headers ?? providerHeaders(provider),
				models: models.map((model) => modelToProviderModel(model, provider, providerId)),
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
