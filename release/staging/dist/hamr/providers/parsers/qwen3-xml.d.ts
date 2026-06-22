/**
 * Qwen3 XML tool-call parser.
 *
 * Parses Qwen3-Coder / Qwen3 XML-format tool calls:
 *
 *   <tool_call>
 *   <function=get_weather>
 *   <parameter=location>San Francisco</parameter>
 *   <parameter=unit>celsius</parameter>
 *   </function>
 *   </tool_call>
 *
 * Reference: vLLM docs/features/tool_calling.md → "Qwen3-Coder Models"
 *   Supported via --tool-call-parser qwen3_xml
 */
import type { ToolCallParser } from "./types.ts";
export declare const qwen3XmlParser: ToolCallParser;
export declare function createQwen3XmlParser(): ToolCallParser;
//# sourceMappingURL=qwen3-xml.d.ts.map