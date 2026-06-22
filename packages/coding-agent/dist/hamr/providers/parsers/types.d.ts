/**
 * Canonical types for Hamr tool-call parsers.
 *
 * Parsers convert model text output into normalized ParsedToolCall records.
 * Inspired by vLLM's tool-call-parser architecture but implemented natively
 * so Hamr does not depend on vLLM runtime normalization.
 */
export interface ParsedToolCall {
    /** Stable call id, either from model output or deterministically generated. */
    id: string;
    /** Function/tool name. */
    name: string;
    /** Parsed arguments object. */
    arguments: Record<string, unknown>;
    /** Raw source text span (the exact substring that was parsed). */
    rawSource?: string;
    /** Parser id that produced this call. */
    parserId?: string;
    /** Recoverable parse warnings. */
    warnings?: string[];
}
export interface ToolCallParseResult {
    /** Whether parsing itself succeeded (even if no calls were found). */
    ok: boolean;
    /** Parser id used. */
    parserId: string;
    /** Parsed calls. This is empty when no calls were detected — that is not an error. */
    calls: ParsedToolCall[];
    /** Non-call content that should remain in the assistant message. */
    content: string;
    /** Errors when ok=false. */
    error?: string;
    /** Recoverable warnings (call-level warnings are on each ParsedToolCall). */
    warnings?: string[];
}
/**
 * A tool-call parser converts raw model text into normalized tool call records.
 *
 * Parsers are stateless single-invocation functions. Streaming buffering
 * is handled by the caller (the provider client), which feeds complete
 * model output to the parser after the stream ends.
 */
export interface ToolCallParser {
    /** Unique parser id, matching vLLM's --tool-call-parser names where practical. */
    readonly id: string;
    /** Human-readable description for config docs. */
    readonly description: string;
    /** Model families this parser is designed for (for docs and auto-detection). */
    readonly modelFamilies: string[];
    /** Parse a complete model response text into canonical calls. */
    parse(content: string): ToolCallParseResult;
}
/**
 * Factory function that creates a parser instance.
 * Matching vLLM's approach, parsers receive a tokenizer only when needed
 * (primarily for parsers that need to decode token IDs). For Hamr's
 * text-based parsing, most parsers use the default no-op factory.
 */
export type ToolCallParserFactory = () => ToolCallParser;
export interface ToolCallParserRegistry {
    /** Register a parser factory under a given id. */
    register(id: string, factory: ToolCallParserFactory): void;
    /** Get a parser by id. Returns undefined if not registered. */
    get(id: string): ToolCallParser | undefined;
    /** List all registered parser ids. */
    listIds(): string[];
    /** List all registered parsers with their descriptions. */
    listParsers(): Array<{
        id: string;
        description: string;
        modelFamilies: string[];
    }>;
    /** Parse content using the parser registered under `id`. */
    parse(id: string, content: string): ToolCallParseResult;
}
/**
 * Conservative auto-detection. Inspects the model id string for substrings
 * matching known model families. Returns undefined when uncertain.
 *
 * Config override always takes priority over auto-detection.
 */
export declare function detectParserId(modelId: string): string | undefined;
//# sourceMappingURL=types.d.ts.map