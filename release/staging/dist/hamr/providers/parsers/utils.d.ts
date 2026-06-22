/**
 * Shared parsing utilities for Hamr tool-call parsers.
 *
 * These are the building blocks used by individual parser implementations.
 * They handle reasoning-tag sanitization, JSON repair, safe value coercion,
 * call id generation, and safe Pythonic argument parsing.
 */
import type { ParsedToolCall } from "./types.ts";
/**
 * Strip <think>/<thinking> reasoning tags from model output.
 * These tags are model-internal and should not affect tool-call parsing.
 */
export declare function sanitizeReasoningTags(content: string): string;
/**
 * Fast-path JSON parse: try native JSON.parse and return immediately on success.
 *
 * This is the hot path — the vast majority of model-generated tool-call blocks
 * are already valid JSON. A single try/catch is orders of magnitude cheaper than
 * running the full repair cascade, and it avoids allocating intermediate strings.
 *
 * Use this directly when you know the source produces clean JSON (e.g., structured
 * output from providers that guarantee valid JSON). Otherwise prefer safeJsonParse.
 */
export declare function fastJsonParse(raw: string): {
    ok: true;
    value: unknown;
} | {
    ok: false;
    error: string;
};
/**
 * Parse JSON with limited repair for common local-model mistakes:
 * - Trailing commas in objects/arrays
 * - Missing closing braces/brackets
 * - Bare keys without quotes in simple cases
 *
 * Uses fastJsonParse as a fast path — returns immediately on valid JSON.
 * Only runs the repair cascade when the initial parse fails.
 *
 * This is intentionally conservative. Complex repair often masks real errors.
 */
export declare function safeJsonParse(raw: string): {
    ok: true;
    value: unknown;
} | {
    ok: false;
    error: string;
};
/**
 * Generate a deterministic-ish call id.
 * Uses the model-provided id if available, otherwise `call_N`.
 */
export declare function generateCallId(provided?: string, index?: number): string;
/** Reset the call id counter (useful for tests). */
export declare function resetCallIdCounter(): void;
/**
 * Coerce a string value to the appropriate JS type for common literal forms.
 * Handles: booleans, null, numbers, quoted strings, nested JSON.
 * Strings that don't match any known literal are returned as-is.
 */
export declare function coerceValue(raw: string): unknown;
export declare function makeCall(name: string, args: Record<string, unknown>, opts?: {
    id?: string;
    index?: number;
    rawSource?: string;
    parserId?: string;
    warnings?: string[];
}): ParsedToolCall;
/**
 * Parse a Pythonic function-call argument string into key-value pairs.
 *
 * Example input: `location="San Francisco", unit='celsius', count=42`
 * Returns: `{ location: "San Francisco", unit: "celsius", count: 42 }`
 *
 * This is a safe tokenizer — it does NOT eval anything.
 */
export declare function parsePythonicArgs(argsStr: string): Record<string, unknown>;
/**
 * Split content into segments separated by tool-call blocks.
 * Returns { before, blocks, between } where:
 * - before is text before the first block
 * - blocks is an array of matched tool-call text
 * - between is text after the last block
 */
export interface DelimitedResult {
    before: string;
    blocks: string[];
    between: string[];
    after: string;
}
export declare function extractDelimitedBlocks(content: string, openTag: string, closeTag: string): DelimitedResult;
/**
 * Extract all content that is NOT inside tool-call delimiters.
 * Used to separate tool calls from prose for transcript rendering.
 */
export declare function extractNonToolContent(content: string, openTag: string, closeTag: string): string;
/**
 * Try to find a pattern anywhere in the text (not just at start).
 * Returns the match and the text before/after for multi-pattern parsing.
 */
export declare function findPattern(content: string, regex: RegExp): {
    match: RegExpMatchArray;
    before: string;
    after: string;
} | null;
//# sourceMappingURL=utils.d.ts.map