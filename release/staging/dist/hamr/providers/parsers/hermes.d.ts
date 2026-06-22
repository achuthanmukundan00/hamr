/**
 * Hermes tool-call parser.
 *
 * Parses Hermes-format tool calls used by NousResearch Hermes models,
 * Qwen2.5 models, and other Hermes-family models:
 *
 *   <tool_call>
 *   {"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}
 *   </tool_call>
 *
 * Each <tool_call> block contains a single JSON object with
 * "name" and "arguments" fields. Multiple blocks = multiple calls.
 *
 * Reference: vLLM docs/features/tool_calling.md → "Qwen Models"
 *   Qwen2.5 chat templates support Hermes-style tool use.
 *   vllm/entrypoints/openai/tool_parsers/hermes_tool_parser.py
 */
import type { ToolCallParser } from "./types.ts";
export declare const hermesParser: ToolCallParser;
export declare function createHermesParser(): ToolCallParser;
//# sourceMappingURL=hermes.d.ts.map