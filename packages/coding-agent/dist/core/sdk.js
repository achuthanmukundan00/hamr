import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { Agent } from "@hamr/agent";
import { clampThinkingLevel, getModel, streamSimple } from "@hamr/ai";
import { getAgentDir } from "../config.js";
import { resolvePath } from "../utils/paths.js";
import { AgentSession } from "./agent-session.js";
import { formatNoModelsAvailableMessage } from "./auth-guidance.js";
import { AuthStorage } from "./auth-storage.js";
import { DEFAULT_THINKING_LEVEL } from "./defaults.js";
import { createExtensionRuntime } from "./extensions/loader.js";
import { convertToLlm } from "./messages.js";
import { ModelRegistry } from "./model-registry.js";
import { findInitialModel } from "./model-resolver.js";
import { mergeProviderAttributionHeaders } from "./provider-attribution.js";
import { DefaultResourceLoader } from "./resource-loader.js";
import { getDefaultSessionDir, SessionManager } from "./session-manager.js";
import { SettingsManager } from "./settings-manager.js";
import { time } from "./timings.js";
import { createBashTool, createCodingTools, createEditTool, createFindTool, createGrepTool, createLsTool, createReadOnlyTools, createReadTool, createWriteTool, withFileMutationQueue, } from "./tools/index.js";
/** Read HamrChildConfig from HAMR_CHILD_CONFIG env var. */
function readChildConfigFromEnv() {
    const configPath = process.env.HAMR_CHILD_CONFIG;
    if (!configPath)
        return undefined;
    try {
        if (!existsSync(configPath))
            return undefined;
        const raw = readFileSync(configPath, "utf-8");
        return JSON.parse(raw);
    }
    catch {
        return undefined;
    }
}
/** Build a Model object from child config fields, falling back to built-in getModel(). */
function buildModelFromChildConfig(config) {
    const fromBuiltin = getModel(config.provider, config.modelId);
    if (fromBuiltin)
        return fromBuiltin;
    // Construct a minimal model for non-built-in providers (e.g. relay).
    if (!config.modelName || !config.modelApi || !config.modelBaseUrl)
        return undefined;
    return {
        provider: config.provider,
        id: config.modelId,
        name: config.modelName,
        api: config.modelApi,
        baseUrl: config.modelBaseUrl,
        reasoning: config.modelReasoning ?? false,
        thinkingLevelMap: config.modelThinkingLevelMap ?? {},
        input: config.modelInput ?? ["text"],
        cost: config.modelCost ?? { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
        contextWindow: config.modelContextWindow ?? 200000,
        maxTokens: config.modelMaxTokens ?? 16384,
        headers: config.modelHeaders,
        compat: config.modelCompat,
    };
}
// Re-exports
export * from "./agent-session-runtime.js";
export { createBashTool, 
// Tool factories (for custom cwd)
createCodingTools, createEditTool, createFindTool, createGrepTool, createLsTool, createReadOnlyTools, createReadTool, createWriteTool, withFileMutationQueue, };
// Helper Functions
function getDefaultAgentDir() {
    return getAgentDir();
}
// ─── Child process session creation (fast path, no file I/O) ────────────────
/**
 * Create an AgentSession from a serialized parent config.
 *
 * When `HAMR_CHILD_CONFIG` is set, this function runs instead of the normal
 * startup path.  It skips:
 *   - Settings file read + lock
 *   - Auth storage read + lock
 *   - Model discovery
 *   - Extension loading (only the tools the parent specified are loaded)
 */
async function createAgentSessionFromChildConfig(config, options) {
    const cwd = resolvePath(config.cwd);
    const agentDir = options.agentDir ? resolvePath(options.agentDir) : getDefaultAgentDir();
    // ── In-memory auth storage with the parent's API key ──────────────────
    const authStorage = AuthStorage.inMemory();
    if (config.apiKey) {
        authStorage.setRuntimeApiKey(config.provider, config.apiKey);
    }
    // Also store provider env + headers via a minimal persisted credential so
    // getProviderEnv() returns them (used by modelRegistry.getApiKeyAndHeaders).
    if (config.apiEnv || config.apiHeaders) {
        authStorage.set(config.provider, {
            type: "api_key",
            key: config.apiKey ?? "not-needed",
            ...(config.apiEnv ? { env: config.apiEnv } : {}),
        });
    }
    // ── In-memory settings manager (no file lock) ─────────────────────────
    const settingsManager = options.settingsManager ?? SettingsManager.inMemory();
    // ── In-memory model registry (built-in models only, no models.json) ───
    const modelRegistry = options.modelRegistry ?? ModelRegistry.inMemory(authStorage);
    // ── In-memory session manager (--no-session) ──────────────────────────
    const sessionManager = options.sessionManager ?? SessionManager.inMemory(cwd);
    // ── Build the model from the parent config ────────────────────────────
    const model = options.model ?? buildModelFromChildConfig(config);
    // ── Thinking level ────────────────────────────────────────────────────
    let thinkingLevel = options.thinkingLevel ?? DEFAULT_THINKING_LEVEL;
    if (model) {
        thinkingLevel = clampThinkingLevel(model, thinkingLevel);
    }
    else {
        thinkingLevel = "off";
    }
    // ── Tool setup ────────────────────────────────────────────────────────
    const allowedToolNames = options.tools ?? config.toolNames;
    const excludedToolNameSet = options.excludeTools ? new Set(options.excludeTools) : undefined;
    const initialActiveToolNames = [...allowedToolNames].filter((name) => !excludedToolNameSet?.has(name));
    // ── Minimal resource loader (no extensions, no model discovery) ───────
    const extensionsResult = {
        extensions: [],
        errors: [],
        runtime: createExtensionRuntime(),
    };
    const noopResourceLoader = {
        getExtensions: () => extensionsResult,
        getSkills: () => ({ skills: [], diagnostics: [] }),
        getPrompts: () => ({ prompts: [], diagnostics: [] }),
        getThemes: () => ({ themes: [], diagnostics: [] }),
        getAgentsFiles: () => ({ agentsFiles: [] }),
        getSystemPrompt: () => config.systemPrompt,
        getAppendSystemPrompt: () => [],
        extendResources: () => { },
        reload: async () => { },
    };
    // ── Stream function (uses the in-memory auth + registry) ──────────────
    const extensionRunnerRef = {};
    const agent = new Agent({
        initialState: {
            systemPrompt: config.systemPrompt,
            model,
            thinkingLevel,
            tools: [],
        },
        convertToLlm: (messages) => convertToLlm(messages),
        streamFn: async (m, context, streamOptions) => {
            const auth = await modelRegistry.getApiKeyAndHeaders(m);
            if (!auth.ok) {
                throw new Error(auth.error);
            }
            const env = auth.env || streamOptions?.env ? { ...(auth.env ?? {}), ...(streamOptions?.env ?? {}) } : undefined;
            return streamSimple(m, context, {
                ...streamOptions,
                apiKey: auth.apiKey,
                env,
                headers: auth.headers,
            });
        },
        sessionId: sessionManager.getSessionId(),
    });
    // ── Save model + thinking level in session ────────────────────────────
    if (model) {
        sessionManager.appendModelChange(model.provider, model.id);
    }
    sessionManager.appendThinkingLevelChange(thinkingLevel);
    // ── Create the session ────────────────────────────────────────────────
    const session = new AgentSession({
        agent,
        sessionManager,
        settingsManager,
        cwd,
        resourceLoader: noopResourceLoader,
        customTools: options.customTools,
        modelRegistry,
        initialActiveToolNames,
        allowedToolNames,
        excludedToolNames: options.excludeTools,
        extensionRunnerRef,
        sessionStartEvent: options.sessionStartEvent,
    });
    // ── Override system prompt (AgentSession._buildRuntime may rebuild it) ─
    if (config.systemPrompt) {
        session.agent.state.systemPrompt = config.systemPrompt;
    }
    return {
        session,
        extensionsResult,
    };
}
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
export async function createAgentSession(options = {}) {
    // ─── Child process fast path: skip all file I/O, locks, and discovery ───
    const childConfig = readChildConfigFromEnv();
    if (childConfig) {
        return createAgentSessionFromChildConfig(childConfig, options);
    }
    const cwd = resolvePath(options.cwd ?? options.sessionManager?.getCwd() ?? process.cwd());
    const agentDir = options.agentDir ? resolvePath(options.agentDir) : getDefaultAgentDir();
    let resourceLoader = options.resourceLoader;
    // Use provided or create AuthStorage and ModelRegistry
    const authPath = options.agentDir ? join(agentDir, "auth.json") : undefined;
    const modelsPath = options.agentDir ? join(agentDir, "models.json") : undefined;
    const authStorage = options.authStorage ?? AuthStorage.create(authPath);
    const modelRegistry = options.modelRegistry ?? ModelRegistry.create(authStorage, modelsPath);
    const settingsManager = options.settingsManager ?? SettingsManager.create(cwd, agentDir);
    const sessionManager = options.sessionManager ?? SessionManager.create(cwd, getDefaultSessionDir(cwd, agentDir));
    if (!resourceLoader) {
        resourceLoader = new DefaultResourceLoader({ cwd, agentDir, settingsManager });
        await resourceLoader.reload();
        time("resourceLoader.reload");
    }
    // Check if session has existing data to restore
    const existingSession = sessionManager.buildSessionContext();
    const hasExistingSession = existingSession.messages.length > 0;
    const hasThinkingEntry = sessionManager.getBranch().some((entry) => entry.type === "thinking_level_change");
    let model = options.model;
    let modelFallbackMessage;
    // If session has data, try to restore model from it
    if (!model && hasExistingSession && existingSession.model) {
        const restoredModel = modelRegistry.find(existingSession.model.provider, existingSession.model.modelId);
        if (restoredModel && modelRegistry.hasConfiguredAuth(restoredModel)) {
            model = restoredModel;
        }
        if (!model) {
            modelFallbackMessage = `Could not restore model ${existingSession.model.provider}/${existingSession.model.modelId}`;
        }
    }
    // If still no model, use findInitialModel (checks settings default, then provider defaults)
    if (!model) {
        const result = await findInitialModel({
            scopedModels: [],
            isContinuing: hasExistingSession,
            defaultProvider: settingsManager.getDefaultProvider(),
            defaultModelId: settingsManager.getDefaultModel(),
            defaultThinkingLevel: settingsManager.getDefaultThinkingLevel(),
            modelRegistry,
        });
        model = result.model;
        if (!model) {
            modelFallbackMessage = formatNoModelsAvailableMessage();
        }
        else if (modelFallbackMessage) {
            modelFallbackMessage += `. Using ${model.provider}/${model.id}`;
        }
    }
    let thinkingLevel = options.thinkingLevel;
    // If session has data, restore thinking level from it
    if (thinkingLevel === undefined && hasExistingSession) {
        thinkingLevel = hasThinkingEntry
            ? existingSession.thinkingLevel
            : (settingsManager.getDefaultThinkingLevel() ?? DEFAULT_THINKING_LEVEL);
    }
    // Fall back to settings default
    if (thinkingLevel === undefined) {
        thinkingLevel = settingsManager.getDefaultThinkingLevel() ?? DEFAULT_THINKING_LEVEL;
    }
    // Clamp to model capabilities
    if (!model) {
        thinkingLevel = "off";
    }
    else {
        thinkingLevel = clampThinkingLevel(model, thinkingLevel);
    }
    const defaultActiveToolNames = ["read", "bash", "edit", "write"];
    const allowedToolNames = options.tools ?? (options.noTools === "all" ? [] : undefined);
    const excludedToolNames = options.excludeTools;
    const excludedToolNameSet = excludedToolNames ? new Set(excludedToolNames) : undefined;
    const initialActiveToolNames = (options.tools ? [...options.tools] : options.noTools ? [] : defaultActiveToolNames).filter((name) => !excludedToolNameSet?.has(name));
    let agent;
    // Create convertToLlm wrapper that filters images if blockImages is enabled (defense-in-depth)
    const convertToLlmWithBlockImages = (messages) => {
        const converted = convertToLlm(messages);
        // Check setting dynamically so mid-session changes take effect
        if (!settingsManager.getBlockImages()) {
            return converted;
        }
        // Filter out ImageContent from all messages, replacing with text placeholder
        return converted.map((msg) => {
            if (msg.role === "user" || msg.role === "toolResult") {
                const content = msg.content;
                if (Array.isArray(content)) {
                    const hasImages = content.some((c) => c.type === "image");
                    if (hasImages) {
                        const filteredContent = content
                            .map((c) => (c.type === "image" ? { type: "text", text: "Image reading is disabled." } : c))
                            .filter((c, i, arr) => 
                        // Dedupe consecutive "Image reading is disabled." texts
                        !(c.type === "text" &&
                            c.text === "Image reading is disabled." &&
                            i > 0 &&
                            arr[i - 1].type === "text" &&
                            arr[i - 1].text === "Image reading is disabled."));
                        return { ...msg, content: filteredContent };
                    }
                }
            }
            return msg;
        });
    };
    const extensionRunnerRef = {};
    agent = new Agent({
        initialState: {
            systemPrompt: "",
            model,
            thinkingLevel,
            tools: [],
        },
        convertToLlm: convertToLlmWithBlockImages,
        streamFn: async (model, context, options) => {
            const auth = await modelRegistry.getApiKeyAndHeaders(model);
            if (!auth.ok) {
                throw new Error(auth.error);
            }
            const env = auth.env || options?.env ? { ...(auth.env ?? {}), ...(options?.env ?? {}) } : undefined;
            const providerRetrySettings = settingsManager.getProviderRetrySettings();
            const httpIdleTimeoutMs = settingsManager.getHttpIdleTimeoutMs();
            // SDKs treat timeout=0 as 0ms (immediate timeout), not "no timeout".
            // Use max int32 to effectively disable the timeout.
            const effectiveTimeoutMs = httpIdleTimeoutMs === 0 ? 2147483647 : httpIdleTimeoutMs;
            const timeoutMs = options?.timeoutMs ?? providerRetrySettings.timeoutMs ?? effectiveTimeoutMs;
            const websocketConnectTimeoutMs = options?.websocketConnectTimeoutMs ?? settingsManager.getWebSocketConnectTimeoutMs();
            return streamSimple(model, context, {
                ...options,
                apiKey: auth.apiKey,
                env,
                timeoutMs,
                websocketConnectTimeoutMs,
                maxRetries: options?.maxRetries ?? providerRetrySettings.maxRetries,
                maxRetryDelayMs: options?.maxRetryDelayMs ?? providerRetrySettings.maxRetryDelayMs,
                headers: mergeProviderAttributionHeaders(model, settingsManager, options?.sessionId, auth.headers, options?.headers),
            });
        },
        onPayload: async (payload, _model) => {
            const runner = extensionRunnerRef.current;
            if (!runner?.hasHandlers("before_provider_request")) {
                return payload;
            }
            return runner.emitBeforeProviderRequest(payload);
        },
        onResponse: async (response, _model) => {
            const runner = extensionRunnerRef.current;
            if (!runner?.hasHandlers("after_provider_response")) {
                return;
            }
            await runner.emit({
                type: "after_provider_response",
                status: response.status,
                headers: response.headers,
            });
        },
        sessionId: sessionManager.getSessionId(),
        transformContext: async (messages) => {
            const runner = extensionRunnerRef.current;
            if (!runner)
                return messages;
            return runner.emitContext(messages);
        },
        steeringMode: settingsManager.getSteeringMode(),
        followUpMode: settingsManager.getFollowUpMode(),
        transport: settingsManager.getTransport(),
        thinkingBudgets: settingsManager.getThinkingBudgets(),
        maxRetryDelayMs: settingsManager.getProviderRetrySettings().maxRetryDelayMs,
    });
    // Restore messages if session has existing data
    if (hasExistingSession) {
        agent.state.messages = existingSession.messages;
        if (!hasThinkingEntry) {
            sessionManager.appendThinkingLevelChange(thinkingLevel);
        }
    }
    else {
        // Save initial model and thinking level for new sessions so they can be restored on resume
        if (model) {
            sessionManager.appendModelChange(model.provider, model.id);
        }
        sessionManager.appendThinkingLevelChange(thinkingLevel);
    }
    const session = new AgentSession({
        agent,
        sessionManager,
        settingsManager,
        cwd,
        scopedModels: options.scopedModels,
        resourceLoader,
        customTools: options.customTools,
        modelRegistry,
        initialActiveToolNames,
        allowedToolNames,
        excludedToolNames,
        extensionRunnerRef,
        sessionStartEvent: options.sessionStartEvent,
    });
    const extensionsResult = resourceLoader.getExtensions();
    return {
        session,
        extensionsResult,
        modelFallbackMessage,
    };
}
//# sourceMappingURL=sdk.js.map