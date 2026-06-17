import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { ThinkingLevel } from "@hamr/agent";
import type { Model } from "@hamr/ai";
import { parse as parseToml } from "toml";
import type { ProviderConfig } from "../core/extensions/types.ts";
import type { ModelRegistry } from "../core/model-registry.ts";

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
	return headers && Object.keys(headers).length > 0 ? headers : undefined;
}

function normalizeThinking(level: HamrThinkingLevel | undefined): ThinkingLevel | undefined {
	if (!level || level === "auto") return undefined;
	return level;
}

async function discoverRelayModels(baseUrl: string): Promise<HamrModelConfig[]> {
	const controller = new AbortController();
	const timeout = setTimeout(() => controller.abort(), 700);
	try {
		const response = await fetch(`${baseUrl.replace(/\/$/, "")}/models`, { signal: controller.signal });
		if (!response.ok) return [];
		const body = (await response.json()) as { data?: Array<{ id?: unknown; name?: unknown }> };
		const models: HamrModelConfig[] = [];
		for (const entry of body.data ?? []) {
			const id = typeof entry.id === "string" ? entry.id : typeof entry.name === "string" ? entry.name : undefined;
			if (id) models.push({ id, display_name: typeof entry.name === "string" ? entry.name : id });
		}
		return models;
	} catch {
		return [];
	} finally {
		clearTimeout(timeout);
	}
}

async function resolveProviderModels(providerId: string, provider: HamrProviderConfig): Promise<HamrModelConfig[]> {
	const configured = provider.models ?? [];
	if (configured.length > 0) return configured;
	if (providerId !== "relay") return [];
	const baseUrl = providerBaseUrl(provider) ?? DEFAULT_RELAY_BASE_URL;
	return discoverRelayModels(baseUrl);
}

function modelToProviderModel(
	model: HamrModelConfig,
	provider: HamrProviderConfig,
): NonNullable<ProviderConfig["models"]>[number] {
	const thinking = model.supports_thinking ?? model.supportsThinking ?? false;
	const contextWindow = model.context_window ?? model.contextWindow ?? 128000;
	const compatibility = provider.compatibility ?? "openai-compatible";
	return {
		id: model.id,
		name: model.display_name ?? model.displayName ?? model.id,
		reasoning: thinking,
		thinkingLevelMap: thinking
			? {
					off: null,
					minimal: "minimal",
					low: "low",
					medium: "medium",
					high: "high",
					xhigh: "xhigh",
				}
			: { off: null },
		input: ((model.supports_vision ?? model.supportsVision) ? ["text", "image"] : ["text"]) as ("text" | "image")[],
		cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
		contextWindow,
		maxTokens: model.max_output_tokens ?? model.maxOutputTokens ?? Math.min(16384, contextWindow),
		compat:
			compatibility === "openai-compatible"
				? {
						supportsUsageInStreaming: false,
						supportsStrictMode: false,
						maxTokensField: "max_tokens",
					}
				: undefined,
	};
}

export async function buildHamrProviderRegistrations(config: HamrStartupConfig): Promise<HamrProviderRegistration[]> {
	const registrations: HamrProviderRegistration[] = [];
	for (const [providerId, provider] of Object.entries(config.providers)) {
		if (provider.enabled === false) continue;
		const compatibility = provider.compatibility ?? "openai-compatible";
		const baseUrl = providerBaseUrl(provider);
		if (!baseUrl) continue;
		const models = await resolveProviderModels(providerId, provider);
		if (models.length === 0) continue;

		const parserByModel = new Map<string, string>();
		for (const model of models) {
			const parser =
				model.tool_call_parser ?? model.toolCallParser ?? provider.tool_call_parser ?? provider.toolCallParser;
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
