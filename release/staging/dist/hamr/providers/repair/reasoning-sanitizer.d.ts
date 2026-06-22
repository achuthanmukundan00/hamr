/**
 * Reasoning sanitizer — strips model thinking/reasoning text from output.
 *
 * Local models frequently leak reasoning text into the visible output stream.
 * This module provides sanitization for:
 * - `<think>...</think>` blocks (Qwen, DeepSeek)
 * - `<thinking>...</thinking>` blocks (Anthropic extended-thinking, various)
 * - ` response/` fenced code blocks (DeepSeek)
 * - DeepSeek reasoning_content leakage into content field
 *
 * Returns sanitized output with a flag indicating whether any reasoning was removed.
 */
export interface SanitizeResult {
    content: string;
    removedReasoning: boolean;
}
/**
 * Sanitize model output by removing reasoning/thinking text.
 *
 * Handles three patterns:
 * 1. `<think>...</think>` — Qwen-style reasoning blocks (with possible attributes)
 * 2. `<thinking>...</thinking>` — More explicit reasoning markers
 * 3. ```response / ```text / ``` reasoning blocks — DeepSeek-style
 *
 * Returns sanitized content and a flag indicating whether anything was removed.
 */
export declare function sanitizeReasoning(content: string): SanitizeResult;
//# sourceMappingURL=reasoning-sanitizer.d.ts.map