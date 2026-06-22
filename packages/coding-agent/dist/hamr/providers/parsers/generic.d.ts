/**
 * Generic tool-call parser (fallback).
 *
 * This is Hamr's existing content-based tool-call parsing, preserved as
 * the "generic" parser. It tries multiple strategies:
 *
 * 1. <tool_call>{"name":"...","arguments":{...}}</tool_call> blocks (Hermes-style)
 * 2. ```json fenced code blocks
 * 3. Bare JSON objects (last resort)
 *
 * This parser is the safe default when no specific parser is configured
 * or auto-detected.
 *
 * It also supports the Qwen3 XML format via alias ('qwen3_coder' maps to 'qwen3_xml').
 */
import type { ToolCallParser } from "./types.ts";
export declare const genericParser: ToolCallParser;
export declare function createGenericParser(): ToolCallParser;
//# sourceMappingURL=generic.d.ts.map