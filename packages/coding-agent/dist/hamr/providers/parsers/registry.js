/**
 * Tool-call parser registry.
 *
 * Maps parser IDs to parser implementations. Supports registration,
 * lookup, listing, and parse dispatch. Designed to mirror vLLM's
 * ToolParserManager but for Hamr's TypeScript runtime.
 *
 * Reference: vLLM docs/features/tool_calling.md and
 *   vllm/entrypoints/openai/tool_parsers/ directory.
 */
import { sanitizeReasoningTags } from "./utils.js";
// ─── Singleton registry ───────────────────────────────────
const parsers = new Map();
export const toolCallParserRegistry = {
    register(id, factory) {
        const normalized = id.trim().toLowerCase();
        if (!normalized)
            throw new Error("parser id must not be empty");
        parsers.set(normalized, factory);
    },
    get(id) {
        const factory = parsers.get(id.trim().toLowerCase());
        return factory?.();
    },
    listIds() {
        return Array.from(parsers.keys()).sort();
    },
    listParsers() {
        return Array.from(parsers.entries())
            .map(([id, factory]) => {
            const parser = factory();
            return { id, description: parser.description, modelFamilies: parser.modelFamilies };
        })
            .sort((a, b) => a.id.localeCompare(b.id));
    },
    parse(id, content) {
        const parser = this.get(id);
        if (!parser) {
            return {
                ok: false,
                parserId: id,
                calls: [],
                content,
                error: `unknown parser: "${id}". Available: ${this.listIds().join(", ")}`,
            };
        }
        const sanitized = sanitizeReasoningTags(content);
        return parser.parse(sanitized);
    },
};
//# sourceMappingURL=registry.js.map