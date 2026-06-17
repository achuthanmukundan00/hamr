/**
 * Config-level types — extracted from old Hamr's llm/types.ts.
 * Bridges Hamr's TOML config to pi's Model type at startup.
 */
import type { ThinkingLevel } from "./schema.js";

export type ThinkingFormat = "openai" | "openrouter" | "deepseek" | "together" | "zai" | "qwen" | "qwen-chat-template";

export type ThinkingLevelMap = Partial<Record<ThinkingLevel, string | null>>;

export interface OpenAiCompatConfig {
	supportsDeveloperRole?: boolean;
	supportsReasoningEffort?: boolean;
	supportsUsageInStreaming?: boolean;
	maxTokensField?: "max_completion_tokens" | "max_tokens";
	requiresToolResultName?: boolean;
	requiresAssistantAfterToolResult?: boolean;
	requiresThinkingAsText?: boolean;
	requiresReasoningContentOnAssistantMessages?: boolean;
	thinkingFormat?: ThinkingFormat;
	cacheControlFormat?: "anthropic";
	supportsStore?: boolean;
	supportsStrictMode?: boolean;
}

export interface AnthropicCompatConfig {
	supportsEagerToolInputStreaming?: boolean;
	supportsLongCacheRetention?: boolean;
	sendSessionAffinityHeaders?: boolean;
	supportsCacheControlOnTools?: boolean;
	forceAdaptiveThinking?: boolean;
	allowEmptySignature?: boolean;
}

export interface ModelCompatConfig {
	openai?: OpenAiCompatConfig;
	anthropic?: AnthropicCompatConfig;
}

export interface NormalizedProviderConfig {
	kind: string;
	baseUrl: string;
	model: string;
	toolCallParser?: string;
	apiKey?: string;
	customHeaders?: Record<string, string>;
	timeoutMs?: number;
	thinkingLevel?: ThinkingLevel;
	maxOutputTokens?: number;
	modelCompat?: ModelCompatConfig;
	thinkingFormat?: ThinkingFormat;
	thinkingLevelMap?: ThinkingLevelMap;
	supportsNativeToolCalls?: boolean;
	maxTokensField?: "max_completion_tokens" | "max_tokens";
	supportsVision?: boolean;
	contextWindow?: number;
}
