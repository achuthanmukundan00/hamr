/**
 * xLAM tool-call parser.
 *
 * Parses xLAM-format tool calls. The xLAM family uses Hermes-compatible
 * XML-style tags with JSON tool call objects:
 *
 *   <tool_call>
 *   {"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}
 *   </tool_call>
 *
 * Reference: vLLM
 *   --tool-call-parser xlam
 *   vllm/entrypoints/openai/tool_parsers/xlam_tool_parser.py
 *
 * Note: xLAM also supports a second format using plain function name then args:
 *   get_weather
 *   {"location": "SF", "unit": "celsius"}
 * This parser handles both formats.
 */
import type { ToolCallParser } from "./types.ts";
export declare const xlamParser: ToolCallParser;
export declare function createXlamParser(): ToolCallParser;
//# sourceMappingURL=xlam.d.ts.map