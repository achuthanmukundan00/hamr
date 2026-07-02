import { type ThinkingLevel } from "@hamr/agent";
import { type Model } from "@hamr/ai";
import { AgentSession } from "./agent-session.ts";
import { AuthStorage } from "./auth-storage.ts";
import type { LoadExtensionsResult, SessionStartEvent, ToolDefinition } from "./extensions/index.ts";
import { ModelRegistry } from "./model-registry.ts";
import type { ResourceLoader } from "./resource-loader.ts";
import { SessionManager } from "./session-manager.ts";
import { SettingsManager } from "./settings-manager.ts";
import { createBashTool, createCodingTools, createEditTool, createFindTool, createGrepTool, createLsTool, createReadOnlyTools, createReadTool, createWriteTool, withFileMutationQueue } from "./tools/index.ts";
/**
 * Serialized parent configuration passed to child hamr processes via
 * `HAMR_CHILD_CONFIG` env var.  Children detect this and skip settings
 * file read, auth storage lock, model discovery, and extension loading.
 */
export interface HamrChildConfig {
    /** API key for the provider (or undefined if auth is header-based). */
    apiKey?: string;
    /** Additional request headers (e.g. for relay auth). */
    apiHeaders?: Record<string, string>;
    /** Provider-scoped environment variables (e.g. OpenRouter referer). */
    apiEnv?: Record<string, string>;
    /** Model provider name (e.g. "anthropic", "openai", "relay"). */
    provider: string;
    /** Model id (e.g. "claude-sonnet-4-5"). */
    modelId: string;
    /** Display name for the model. */
    modelName?: string;
    /** API type: "openai-completions" | "anthropic-messages" | etc. */
    modelApi?: string;
    /** Base URL for the provider endpoint. */
    modelBaseUrl?: string;
    /** Context window size. */
    modelContextWindow?: number;
    /** Max output tokens. */
    modelMaxTokens?: number;
    /** Whether the model supports reasoning/thinking. */
    modelReasoning?: boolean;
    /** Accepted input modalities. */
    modelInput?: string[];
    /** Per-token cost info. */
    modelCost?: {
        input: number;
        output: number;
        cacheRead: number;
        cacheWrite: number;
    };
    /** Per-model request headers. */
    modelHeaders?: Record<string, string>;
    /** Thinking level map (e.g. { medium: "medium", xhigh: null }). */
    modelThinkingLevelMap?: Record<string, string | null>;
    /** Provider/model compat config. */
    modelCompat?: Record<string, unknown>;
    /** Tool names the child is allowed to use. */
    toolNames: string[];
    /** Pre-built system prompt from the parent. */
    systemPrompt: string;
    /** Working directory. */
    cwd: string;
    /** Remaining subagent tree budget. */
    treeBudgetRemaining: number;
    /** Recursion depth of subagent. */
    subagentDepth?: number;
}
export interface CreateAgentSessionOptions {
    /** Working directory for project-local discovery. Default: process.cwd() */
    cwd?: string;
    /** Global config directory. Default: ~/.hamr/agent */
    agentDir?: string;
    /** Auth storage for credentials. Default: AuthStorage.create(agentDir/auth.json) */
    authStorage?: AuthStorage;
    /** Model registry. Default: ModelRegistry.create(authStorage, agentDir/models.json) */
    modelRegistry?: ModelRegistry;
    /** Model to use. Default: from settings, else first available */
    model?: Model<any>;
    /** Thinking level. Default: from settings, else 'medium' (clamped to model capabilities) */
    thinkingLevel?: ThinkingLevel;
    /** Models available for cycling (Ctrl+P in interactive mode) */
    scopedModels?: Array<{
        model: Model<any>;
        thinkingLevel?: ThinkingLevel;
    }>;
    /**
     * Optional default tool suppression mode when no explicit allowlist is provided.
     *
     * - "all": start with no tools enabled
     * - "builtin": disable the default built-in tools (read, bash, edit, write)
     *   but keep extension/custom tools enabled
     */
    noTools?: "all" | "builtin";
    /**
     * Optional allowlist of tool names.
     *
     * When omitted, pi enables the default built-in tools (read, bash, edit, write)
     * and leaves extension/custom tools enabled unless `noTools` changes that default.
     * When provided, only the listed tool names are enabled.
     */
    tools?: string[];
    /** Optional denylist of tool names to disable. Applies after `tools` when both are provided. */
    excludeTools?: string[];
    /** Custom tools to register (in addition to built-in tools). */
    customTools?: ToolDefinition[];
    /** Resource loader. When omitted, DefaultResourceLoader is used. */
    resourceLoader?: ResourceLoader;
    /** Session manager. Default: SessionManager.create(cwd) */
    sessionManager?: SessionManager;
    /** Settings manager. Default: SettingsManager.create(cwd, agentDir) */
    settingsManager?: SettingsManager;
    /** Session start event metadata for extension runtime startup. */
    sessionStartEvent?: SessionStartEvent;
}
/** Result from createAgentSession */
export interface CreateAgentSessionResult {
    /** The created session */
    session: AgentSession;
    /** Extensions result (for UI context setup in interactive mode) */
    extensionsResult: LoadExtensionsResult;
    /** Warning if session was restored with a different model than saved */
    modelFallbackMessage?: string;
}
export * from "./agent-session-runtime.ts";
export type { ExtensionAPI, ExtensionCommandContext, ExtensionContext, ExtensionFactory, SlashCommandInfo, SlashCommandSource, ToolDefinition, } from "./extensions/index.ts";
export type { PromptTemplate } from "./prompt-templates.ts";
export type { Skill } from "./skills.ts";
export type { Tool } from "./tools/index.ts";
export { createBashTool, createCodingTools, createEditTool, createFindTool, createGrepTool, createLsTool, createReadOnlyTools, createReadTool, createWriteTool, withFileMutationQueue, };
/**
 * Create an AgentSession with the specified options.
 *
 * @example
 * ```typescript
 * // Minimal - uses defaults
 * const { session } = await createAgentSession();
 *
 * // With explicit model
 * import { getModel } from '@hamr/ai';
 * const { session } = await createAgentSession({
 *   model: getModel('anthropic', 'claude-opus-4-5'),
 *   thinkingLevel: 'high',
 * });
 *
 * // Continue previous session
 * const { session, modelFallbackMessage } = await createAgentSession({
 *   continueSession: true,
 * });
 *
 * // Full control
 * const loader = new DefaultResourceLoader({
 *   cwd: process.cwd(),
 *   agentDir: getAgentDir(),
 *   settingsManager: SettingsManager.create(),
 * });
 * await loader.reload();
 * const { session } = await createAgentSession({
 *   model: myModel,
 *   tools: ["read", "bash"],
 *   resourceLoader: loader,
 *   sessionManager: SessionManager.inMemory(),
 * });
 * ```
 */
export declare function createAgentSession(options?: CreateAgentSessionOptions): Promise<CreateAgentSessionResult>;
//# sourceMappingURL=sdk.d.ts.map