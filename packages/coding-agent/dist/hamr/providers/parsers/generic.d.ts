/**
 * Generic tool-call parser (fallback).
 *
 * Handles both JSON and XML tool-call formats so unrecognised model families
 * (Apodex, Qwen derivatives, …) are not silently broken.
 *
 * Strategies (tried in order):
 * 1. <tool_call> blocks → JSON first (Hermes-style), then XML (Qwen3-style)
 * 2. ```json fenced code blocks
 * 3. Bare JSON objects (last resort)
 */
import type { ToolCallParser } from "./types.ts";
export declare const genericParser: ToolCallParser;
export declare function createGenericParser(): ToolCallParser;
//# sourceMappingURL=generic.d.ts.map