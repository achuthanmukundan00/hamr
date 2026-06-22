/**
 * Shared JSON-in-tags parser factory.
 *
 * Many model families use the same basic format: tool calls wrapped in
 * XML-style tags with JSON objects inside. This module provides a
 * factory for creating parsers for these families.
 *
 * Format:
 *   <tool_call>
 *   {"name": "func_name", "arguments": {"key": "value"}}
 *   </tool_call>
 *
 * Used by: Granite, InternLM, FunctionGemma, OLMo3, Jamba, MiniMax,
 * Kimi K2, Hunyuan, LongCat, GigaChat, and others.
 *
 * Reference: vLLM
 *   vllm/entrypoints/openai/tool_parsers/ — various parsers
 */
import type { ToolCallParser } from "./types.ts";
export interface JsonInTagsParserConfig {
    id: string;
    description: string;
    modelFamilies: string[];
    /** Open tag regex or string (default: '<tool_call>'). */
    openTag?: string;
    /** Close tag regex or string (default: '</tool_call>'). */
    closeTag?: string;
    /** Key for function name in JSON object (default: 'name'). */
    nameKey?: string;
    /** Key for arguments in JSON object (default: 'arguments'). */
    argsKey?: string;
}
export declare function createJsonInTagsParser(config: JsonInTagsParserConfig): ToolCallParser;
export declare const graniteParser: ToolCallParser;
export declare const granite4Parser: ToolCallParser;
export declare const granite20bFcParser: ToolCallParser;
export declare const internlmParser: ToolCallParser;
export declare const functionGemmaParser: ToolCallParser;
/**
 * OLMo3 tool-call parser.
 *
 * OLMo3 wraps multiple tool calls in <function_calls>...</function_calls>
 * with individual <function_call> entries:
 *
 *   <function_calls>
 *   <function_call>
 *   {"name": "get_weather", "arguments": {"location": "SF"}}
 *   </function_call>
 *   <function_call>
 *   {"name": "get_time", "arguments": {"timezone": "PST"}}
 *   </function_call>
 *   </function_calls>
 *
 * Reference: vLLM
 *   --tool-call-parser olmo3
 *   vllm/entrypoints/openai/tool_parsers/olmo3_tool_parser.py
 */
export declare const olmo3Parser: ToolCallParser;
export declare const jambaParser: ToolCallParser;
export declare const minimaxParser: ToolCallParser;
export declare const kimiK2Parser: ToolCallParser;
export declare const hunyuanA13bParser: ToolCallParser;
export declare const longcatParser: ToolCallParser;
/**
 * GigaChat 3 tool-call parser.
 *
 * GigaChat 3 uses a custom format with <function> tags:
 *
 *   <function=func_name>{"key": "value"}</function>
 *
 * Reference: vLLM
 *   --tool-call-parser gigachat3
 */
export declare const gigachat3Parser: ToolCallParser;
/**
 * OpenAI tool-call parser.
 *
 * This is a no-op parser for vLLM's `--tool-call-parser openai` mode.
 * In this mode, vLLM returns native OpenAI-format tool_calls in the
 * API response, so no text parsing is needed.
 *
 * Hamr handles OpenAI tool_calls natively via parseOpenAIToolCallsResult
 * in the client. This parser is a passthrough for explicit config:
 *   tool_call_parser = "openai"
 */
export declare const openaiPassthroughParser: ToolCallParser;
export declare function createGraniteParser(): ToolCallParser;
export declare function createGranite4Parser(): ToolCallParser;
export declare function createGranite20bFcParser(): ToolCallParser;
export declare function createInternlmParser(): ToolCallParser;
export declare function createFunctionGemmaParser(): ToolCallParser;
export declare function createOlmo3Parser(): ToolCallParser;
export declare function createJambaParser(): ToolCallParser;
export declare function createMinimaxParser(): ToolCallParser;
export declare function createKimiK2Parser(): ToolCallParser;
export declare function createHunyuanA13bParser(): ToolCallParser;
export declare function createLongcatParser(): ToolCallParser;
export declare function createGigachat3Parser(): ToolCallParser;
export declare function createOpenaiPassthroughParser(): ToolCallParser;
//# sourceMappingURL=json-in-tags.d.ts.map