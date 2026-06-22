/**
 * Relay model auto-detection.
 *
 * The relay is just an OpenAI-compatible HTTP endpoint (llama.cpp / vLLM /
 * SGLang style). Hamr treats it as a native provider: at startup it queries
 * the endpoint's `GET /v1/models` and discovers whatever models the relay is
 * currently serving, so the user never has to hand-list them.
 *
 * Ported from synax's `probe-models.ts`: scans the many context-window field
 * names different OpenAI-compatible servers emit, reads vision capability from
 * the `capabilities` array (llama.cpp emits `["completion", "multimodal"]`
 * when `--mmproj` is loaded), and derives a readable display name.
 *
 * All failures are swallowed — an unreachable endpoint yields an empty list,
 * never a crash.
 */
// Common field names used by various OpenAI-compatible servers for context window.
// Order roughly by likelihood: llama.cpp forks (max_context_length), vLLM/SGLang
// (max_model_len), generic (context_length), and others.
const CONTEXT_WINDOW_FIELDS = [
    "max_context_length",
    "max_model_len",
    "context_length",
    "max_total_tokens",
    "total_tokens_capacity",
    "n_ctx",
    "max_position_embeddings",
    "model_max_length",
    "max_seq_len",
    "max_sequence_length",
];
const KNOWN_THINKING_LEVELS = ["off", "on", "low", "medium", "high", "xhigh"];
// Discovery is best-effort and must never block startup for long.
const DISCOVERY_TIMEOUT_MS = 5000;
function buildHeaders(apiKey, customHeaders) {
    const headers = {
        Accept: "application/json",
        "User-Agent": "hamr/1.0",
    };
    if (apiKey) {
        headers.Authorization = `Bearer ${apiKey}`;
    }
    if (customHeaders) {
        for (const [key, value] of Object.entries(customHeaders)) {
            headers[key] = value;
        }
    }
    return headers;
}
function extractContextWindow(entry) {
    // Check top-level fields first.
    for (const field of CONTEXT_WINDOW_FIELDS) {
        const value = entry[field];
        if (typeof value === "number" && Number.isFinite(value) && value > 0) {
            return value;
        }
        if (typeof value === "string") {
            const parsed = Number.parseInt(value, 10);
            if (!Number.isNaN(parsed) && parsed > 0)
                return parsed;
        }
    }
    // Some servers (e.g. llama.cpp) nest context-length fields inside meta.
    const meta = entry.meta;
    if (meta && typeof meta === "object") {
        for (const field of CONTEXT_WINDOW_FIELDS) {
            const value = meta[field];
            if (typeof value === "number" && Number.isFinite(value) && value > 0) {
                return value;
            }
            if (typeof value === "string") {
                const parsed = Number.parseInt(value, 10);
                if (!Number.isNaN(parsed) && parsed > 0)
                    return parsed;
            }
        }
    }
    return undefined;
}
/**
 * Derive a human-readable display name from a model ID, stripping common
 * file extensions and quantization suffixes (gguf / IQ / Q*_K variants).
 */
function deriveDisplayName(id) {
    const cleaned = id
        .replace(/\.gguf$/i, "")
        .replace(/[-_]IQ[23456]_\w+$/i, "")
        .replace(/[-_]Q[234568]_K_\w+$/i, "")
        .replace(/[-_](f16|f32|q4_0|q4_1|q5_0|q5_1|q8_0)$/i, "");
    return cleaned.replace(/[-_]/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}
function stringList(value) {
    if (!Array.isArray(value))
        return [];
    return value.map((item) => String(item).toLowerCase());
}
function booleanField(entry, fields) {
    for (const field of fields) {
        const value = entry[field];
        if (typeof value === "boolean")
            return value;
        if (typeof value === "string") {
            const lower = value.toLowerCase();
            if (["true", "yes", "1"].includes(lower))
                return true;
            if (["false", "no", "0"].includes(lower))
                return false;
        }
    }
    return undefined;
}
function extractSupportsVision(entry) {
    const explicit = booleanField(entry, [
        "supports_vision",
        "supportsVision",
        "vision",
        "multimodal",
        "supports_images",
        "supportsImages",
    ]);
    if (explicit !== undefined)
        return explicit;
    const meta = entry.meta && typeof entry.meta === "object" ? entry.meta : undefined;
    if (meta) {
        const metaExplicit = booleanField(meta, [
            "supports_vision",
            "supportsVision",
            "vision",
            "multimodal",
            "supports_images",
            "supportsImages",
        ]);
        if (metaExplicit !== undefined)
            return metaExplicit;
    }
    const haystack = [
        ...stringList(entry.capabilities),
        ...stringList(entry.modalities),
        ...stringList(entry.input_modalities),
        ...stringList(entry.inputModalities),
        ...stringList(entry.features),
        ...(meta ? stringList(meta.capabilities) : []),
        ...(meta ? stringList(meta.modalities) : []),
        ...(meta ? stringList(meta.input_modalities) : []),
        ...(meta ? stringList(meta.inputModalities) : []),
        ...(meta ? stringList(meta.features) : []),
    ];
    if (haystack.some((item) => item === "multimodal" || item === "vision" || item === "image" || item === "images")) {
        return true;
    }
    if (haystack.some((item) => item === "text-only" || item === "text_only")) {
        return false;
    }
    return undefined;
}
/**
 * Fetch available models from an OpenAI-compatible `GET /v1/models` endpoint.
 *
 * Returns the discovered models, or an empty array if the endpoint could not
 * be reached or returned an unrecognized payload. Never throws.
 */
export async function discoverRelayModels(baseUrl, apiKey, customHeaders) {
    const cleanBaseUrl = baseUrl.replace(/\/+$/, "");
    const headers = buildHeaders(apiKey, customHeaders);
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), DISCOVERY_TIMEOUT_MS);
    try {
        const res = await fetch(`${cleanBaseUrl}/models`, {
            method: "GET",
            headers,
            signal: controller.signal,
        });
        if (!res.ok)
            return [];
        const body = (await res.json());
        // Standard OpenAI /v1/models response: { object: "list", data: [...] }
        // Some servers return a bare array instead.
        const data = body?.data;
        const entries = Array.isArray(data)
            ? data
            : Array.isArray(body)
                ? body
                : undefined;
        if (!entries)
            return [];
        const models = [];
        for (const entry of entries) {
            const rawId = typeof entry.id === "string" ? entry.id : typeof entry.name === "string" ? entry.name : "";
            const id = rawId.trim();
            if (!id)
                continue;
            const contextWindow = extractContextWindow(entry);
            // Prefer server-provided display_name / name, fall back to deriving from ID.
            const serverName = (typeof entry.display_name === "string" && entry.display_name.trim()) ||
                (typeof entry.name === "string" && entry.name.trim()) ||
                "";
            const displayName = serverName ? serverName : deriveDisplayName(id);
            const supportsVision = extractSupportsVision(entry);
            const supportsThinking = typeof entry.supports_thinking === "boolean" ? entry.supports_thinking : false;
            const apiLevels = Array.isArray(entry.thinking_levels)
                ? entry.thinking_levels.map((l) => String(l)).filter((l) => KNOWN_THINKING_LEVELS.includes(l))
                : [];
            // When the server says thinking is supported but doesn't advertise
            // specific levels, default to boolean on/off (standard for local models).
            const thinkingLevels = apiLevels.length > 0 ? apiLevels : supportsThinking ? ["off", "on"] : [];
            const maxOutputTokens = typeof entry.max_output_tokens === "number" && entry.max_output_tokens > 0
                ? entry.max_output_tokens
                : undefined;
            models.push({
                id,
                displayName,
                contextWindow,
                maxOutputTokens,
                supportsThinking,
                thinkingLevels,
                supportsVision,
            });
        }
        return models;
    }
    catch {
        return [];
    }
    finally {
        clearTimeout(timeout);
    }
}
//# sourceMappingURL=relay-provider.js.map