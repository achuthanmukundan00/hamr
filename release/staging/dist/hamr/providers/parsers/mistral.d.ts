/**
 * Mistral tool-call parser.
 *
 * Parses Mistral-format tool calls:
 *
 *   [TOOL_CALLS][{"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}]
 *
 * The model outputs a [TOOL_CALLS] prefix followed by a JSON array of
 * tool call objects. Each object has "name" and "arguments" fields.
 * Multiple calls can appear in the same JSON array.
 *
 * Variants:
 *   - mistral: standard Mistral format
 *
 * Reference: vLLM docs/features/tool_calling.md
 *   --tool-call-parser mistral
 *   vllm/entrypoints/openai/tool_parsers/mistral_tool_parser.py
 */
import type { ToolCallParser } from "./types.ts";
export declare const mistralParser: ToolCallParser;
export declare function createMistralParser(): ToolCallParser;
//# sourceMappingURL=mistral.d.ts.map