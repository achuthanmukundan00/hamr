import type { ThinkingLevel } from "@hamr/agent";
import { type Model } from "@hamr/ai";
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
    cost?: {
        input: number;
        output: number;
        cacheRead: number;
        cacheWrite: number;
    };
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
export declare function loadHamrStartupConfig(cwd?: string): HamrStartupConfig;
/**
 * Whether the given provider id should be treated as a cloud provider.
 *
 * Configured providers (relay, local LM endpoints) are local by convention and
 * default to non-cloud unless they set `cloud: true`. Providers that are not in
 * the hamr config at all are the built-in cloud providers (anthropic, openai,
 * …), so they are treated as cloud. Used to gate parallel subagent dispatch.
 */
export declare function isCloudProvider(config: HamrStartupConfig, providerId: string): boolean;
export declare function buildHamrProviderRegistrations(config: HamrStartupConfig): Promise<HamrProviderRegistration[]>;
export declare function getHamrDefaultModel(config: HamrStartupConfig, modelRegistry: ModelRegistry): {
    model: Model<any>;
    thinkingLevel?: ThinkingLevel;
} | undefined;
export {};
//# sourceMappingURL=startup-config.d.ts.map