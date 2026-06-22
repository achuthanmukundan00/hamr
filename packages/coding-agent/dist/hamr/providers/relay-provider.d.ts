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
/** A model discovered from a relay / OpenAI-compatible endpoint. */
export interface DiscoveredRelayModel {
    id: string;
    displayName: string;
    contextWindow?: number;
    maxOutputTokens?: number;
    supportsThinking: boolean;
    thinkingLevels: string[];
    supportsVision?: boolean;
}
/**
 * Fetch available models from an OpenAI-compatible `GET /v1/models` endpoint.
 *
 * Returns the discovered models, or an empty array if the endpoint could not
 * be reached or returned an unrecognized payload. Never throws.
 */
export declare function discoverRelayModels(baseUrl: string, apiKey?: string, customHeaders?: Record<string, string>): Promise<DiscoveredRelayModel[]>;
//# sourceMappingURL=relay-provider.d.ts.map