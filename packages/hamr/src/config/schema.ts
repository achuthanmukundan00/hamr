/**
 * Hamr config schema — multi-provider, model, skills, MCP, thinking levels.
 *
 * This extends the existing single-provider config in project.ts with
 * the new multi-provider format. Both formats coexist; the effective
 * config layer resolves the active provider from whichever format is used.
 */

// ─── Provider types ────────────────────────────────────────

export type ProviderCompatibility = "openai-compatible" | "anthropic-compatible";

export type ThinkingLevel = "off" | "on" | "low" | "medium" | "high" | "xhigh" | "auto";

export interface ModelConfig {
	id: string;
	displayName?: string;
	display_name?: string;
	contextWindow?: number;
	context_window?: number;
	supportsThinking?: boolean;
	supports_thinking?: boolean;
	thinkingLevels?: ThinkingLevel[];
	thinking_levels?: ThinkingLevel[];
	defaultThinkingLevel?: ThinkingLevel;
	default_thinking?: ThinkingLevel;
	/** Per-model max output tokens. When unset the client default (8192) is used. */
	maxOutputTokens?: number;
	max_output_tokens?: number;
	/** Whether this model supports vision / image inputs. */
	supportsVision?: boolean;
	supports_vision?: boolean;
	// ─── Per-model compat (from models.json / TOML config) ───
	/** Tool-call parser ID override (e.g. 'qwen3_xml', 'deepseek', 'llama3_json'). */
	toolCallParser?: string;
	tool_call_parser?: string;
	/** Thinking parameter format for this model. */
	thinkingFormat?: import("./types.js").ThinkingFormat;
	thinking_format?: import("./types.js").ThinkingFormat;
	/** Maps Hamr thinking levels to provider-specific values; null = unsupported. */
	thinkingLevelMap?: Partial<Record<ThinkingLevel, string | null>>;
	thinking_level_map?: Partial<Record<ThinkingLevel, string | null>>;
	/** OpenAI/Anthropic compat flags (merged with provider-level). */
	compat?: import("./types.js").ModelCompatConfig;
	/** Whether model supports native OpenAI tool_calls. */
	supportsNativeToolCalls?: boolean;
	supports_native_tool_calls?: boolean;
	/** Max tokens field preference. */
	maxTokensField?: "max_completion_tokens" | "max_tokens";
	max_tokens_field?: "max_completion_tokens" | "max_tokens";
}

export interface ProviderConfig {
	id: string;
	name?: string;
	compatibility: ProviderCompatibility;
	enabled?: boolean;
	baseUrl?: string;
	base_url?: string;
	apiKeyEnv?: string;
	api_key_env?: string;
	apiKey?: string;
	api_key?: string;
	headers?: Record<string, string>;
	models: ModelConfig[];
	// ─── Provider-level compat & discovery ───
	/** Provider-level compat defaults (merged with per-model compat). */
	compat?: import("./types.js").ModelCompatConfig;
	/** Auto-discover models from /v1/models endpoint. */
	autoDiscover?: boolean;
	auto_discover?: boolean;
	/** URL path override for model discovery (default: /v1/models). */
	modelsEndpoint?: string;
	models_endpoint?: string;
	/** Per-model overrides for built-in models (keeps built-in list, overrides specific fields). */
	modelOverrides?: Record<string, Partial<ModelConfig>>;
	model_overrides?: Record<string, Partial<ModelConfig>>;
}

// ─── Active config ─────────────────────────────────────────

export interface ActiveConfig {
	provider?: string;
	model?: string;
	thinking?: ThinkingLevel;
}

// ─── Skills config ─────────────────────────────────────────

export interface SkillsConfig {
	enabled?: string[];
	disabled?: string[];
}

// ─── MCP config ────────────────────────────────────────────

export interface McpServerConfig {
	enabled?: boolean;
	command: string;
	args?: string[];
	env?: Record<string, string>;
}

export interface McpConfig {
	servers?: Record<string, McpServerConfig>;
}

// ─── TUI config ────────────────────────────────────────────

export interface TuiConfig {
	/** Enable SGR mouse tracking for app-managed wheel scrolling. Default false. */
	mouse?: boolean;
	/** Use alternate screen buffer. Default false — primary buffer allows native
      terminal text selection with scrollback (select + shift-click across
      scrolled content). Set to true for clean alternate-screen rendering. */
	alternateScreen?: boolean;
	alternate_screen?: boolean;
	/** Reduce TUI frame rate, event churn, and live nodes for cmux/high-load sessions. */
	cmuxMode?: boolean;
	cmux_mode?: boolean;
	/** If true, only shows the glyph for active thinking artifacts. Default false. */
	hideThinking?: boolean;
	hide_thinking?: boolean;
	/** Active theme name (e.g. "pinkOut", "kawaii"). Defaults to "default". */
	theme?: string;
}

// ─── Compaction settings ──────────────────────────────

export interface CompactionSettings {
	/** Enable compaction. Default true. */
	enabled?: boolean;
	/** Tokens reserved for prompt + LLM response during summarization. Default 16384. */
	reserveTokens?: number;
	reserve_tokens?: number;
	/** Recent tokens to keep uncompacted. Default 20000. */
	keepRecentTokens?: number;
	keep_recent_tokens?: number;
}

// ─── Retry settings ───────────────────────────────────

export interface RetrySettings {
	/** Enable provider-level retry. Default true. */
	enabled?: boolean;
	/** Max retry attempts. Default 3. */
	maxRetries?: number;
	max_retries?: number;
	/** Base delay for exponential backoff in ms. Default 2000. */
	baseDelayMs?: number;
	base_delay_ms?: number;
	/** Provider request timeout in ms. */
	timeoutMs?: number;
	timeout_ms?: number;
	/** Max server-requested delay before failing in ms. Default 60000. */
	maxRetryDelayMs?: number;
	max_retry_delay_ms?: number;
}

// ─── Resolved settings ────────────────────────────────

export interface ResolvedCompactionSettings {
	enabled: boolean;
	reserveTokens: number;
	keepRecentTokens: number;
}

export interface ResolvedRetrySettings {
	enabled: boolean;
	maxRetries: number;
	baseDelayMs: number;
	timeoutMs: number;
	maxRetryDelayMs: number;
}

export interface ResolvedTuiConfig {
	mouse: boolean;
	alternateScreen: boolean;
	cmuxMode: boolean;
	hideThinking: boolean;
	theme?: string;
}

// ─── Full project config (extended) ────────────────────────

export interface HamrConfig {
	active?: ActiveConfig;
	provider?: Record<string, unknown>; // legacy single-provider
	providers?: Record<string, ProviderConfig>;
	skills?: SkillsConfig;
	mcp?: McpConfig;
	tui?: TuiConfig;
	/** @deprecated No longer consumed by the TUI. Kept for config compatibility. */
	coreVisualProfile?: string;
}

// ─── Resolved / effective types ────────────────────────────

export interface ResolvedModelConfig {
	id: string;
	displayName?: string;
	contextWindow?: number;
	supportsThinking: boolean;
	thinkingLevels: ThinkingLevel[];
	defaultThinkingLevel?: ThinkingLevel;
	/** Per-model max output tokens. When unset the client default (8192) is used. */
	maxOutputTokens?: number;
	/** Whether this model supports vision / image inputs (multimodal). */
	supportsVision: boolean;
	// ─── Per-model compat (resolved from config) ───
	/** Tool-call parser ID override. */
	toolCallParser?: string;
	/** Thinking parameter format. */
	thinkingFormat?: import("./types.js").ThinkingFormat;
	/** Maps Hamr thinking levels to provider-specific values. */
	thinkingLevelMap?: Partial<Record<ThinkingLevel, string | null>>;
	/** OpenAI/Anthropic compat flags (resolved from provider + model level). */
	compat?: import("./types.js").ModelCompatConfig;
	/** Whether model supports native OpenAI tool_calls. */
	supportsNativeToolCalls?: boolean;
	/** Max tokens field preference. */
	maxTokensField?: "max_completion_tokens" | "max_tokens";
}

export interface ResolvedProviderConfig {
	id: string;
	name: string;
	compatibility: ProviderCompatibility;
	enabled: boolean;
	baseUrl: string;
	apiKeyEnv?: string;
	apiKey?: string;
	headers: Record<string, string>;
	models: ResolvedModelConfig[];
}

export interface ResolvedSkillsConfig {
	enabled: string[];
	disabled: string[];
}

export interface ResolvedMcpConfig {
	servers: Record<string, ResolvedMcpServerConfig>;
}

export interface ResolvedMcpServerConfig {
	enabled: boolean;
	command: string;
	args: string[];
	env: Record<string, string>;
}

export interface ResolvedActiveConfig {
	provider: string;
	model: string;
	thinking: ThinkingLevel;
}

export interface EffectiveHamrConfig {
	active: ResolvedActiveConfig;
	providers: Record<string, ResolvedProviderConfig>;
	skills: ResolvedSkillsConfig;
	mcp: ResolvedMcpConfig;
	tui?: ResolvedTuiConfig;
	/** @deprecated No longer consumed by the TUI. Kept for config compatibility. */
	coreVisualProfile?: string;
	/** The source path that provided the effective config, or null for defaults. */
	source: string | null;
	/** Validation errors encountered during loading/merging. */
	errors: string[];
}

// ─── Config source tracking ────────────────────────────────

export type ConfigSource = "default" | "global" | "local";

export interface ConfigSourceInfo {
	source: ConfigSource;
	path: string | null;
}
