/**
 * DeepSeek V3 / V3.1 tool-call parser.
 *
 * Parses DeepSeek tool-call format. DeepSeek models output tool calls in
 * a format similar to Hermes, using XML-style tags with JSON inside:
 *
 *   <tool_call>
 *   {"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}
 *   </tool_call>
 *
 * DeepSeek V3.1 may also use a <｜tool▁call▁begin｜>...<｜tool▁call▁end｜> format
 * with a special token prefix.
 *
 * DeepSeek reasoning models (R1) may emit tool calls inside <think> blocks
 * — the sanitizeReasoningTags step strips those before parsing.
 *
 * Reference: vLLM
 *   --tool-call-parser deepseek_v3
 *   --tool-call-parser deepseek_v31
 *   vllm/entrypoints/openai/tool_parsers/deepseek_v3_tool_parser.py
 *   vllm/entrypoints/openai/tool_parsers/deepseek_v31_tool_parser.py
 */
import type { ToolCallParser } from "./types.ts";
export declare const deepseekV3Parser: ToolCallParser;
export declare const deepseekV31Parser: ToolCallParser;
export declare function createDeepseekV3Parser(): ToolCallParser;
export declare function createDeepseekV31Parser(): ToolCallParser;
//# sourceMappingURL=deepseek.d.ts.map