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
import type { ToolCallParserRegistry } from "./types.ts";
export declare const toolCallParserRegistry: ToolCallParserRegistry;
//# sourceMappingURL=registry.d.ts.map