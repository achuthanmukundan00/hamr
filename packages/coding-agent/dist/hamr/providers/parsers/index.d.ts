/**
 * Hamr native tool-call parsers — index and registration.
 *
 * Registers all vLLM-equivalent tool-call parsers into the singleton registry.
 * Import this module once at startup to enable all parsers.
 *
 * Parser IDs match vLLM's --tool-call-parser values where practical.
 * Aliases are registered only for backward compatibility (e.g., 'qwen3_coder' → 'qwen3_xml').
 */
export declare function ensureParsersRegistered(): void;
export { toolCallParserRegistry } from "./registry.ts";
export type { ParsedToolCall, ToolCallParseResult, ToolCallParser, ToolCallParserFactory, ToolCallParserRegistry, } from "./types.ts";
export { detectParserId } from "./types.ts";
export { coerceValue, extractDelimitedBlocks, extractNonToolContent, fastJsonParse, generateCallId, makeCall, parsePythonicArgs, resetCallIdCounter, safeJsonParse, sanitizeReasoningTags, } from "./utils.ts";
//# sourceMappingURL=index.d.ts.map