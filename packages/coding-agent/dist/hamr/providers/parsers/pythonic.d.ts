/**
 * Pythonic tool-call parser.
 *
 * Parses Python-list-format tool calls used by models that generate
 * Python syntax for function calls:
 *
 *   [get_weather(city='San Francisco', metric='celsius'),
 *    get_weather(city='Seattle', metric='celsius')]
 *
 * Also supports bare calls without the list wrapper:
 *
 *   get_weather(city='San Francisco', metric='celsius')
 *
 * This parser uses a safe tokenizer — it does NOT eval anything.
 * Supports parallel tool calls (multiple functions in one list).
 *
 * Variants:
 *   - pythonic: general Pythonic list format
 *   - llama4_pythonic: subset with Llama-4-specific handling
 *
 * Reference: vLLM docs/features/tool_calling.md → "Models with Pythonic Tool Calls"
 *   --tool-call-parser pythonic
 *   vllm/entrypoints/openai/tool_parsers/pythonic_tool_parser.py
 */
import type { ToolCallParser } from "./types.ts";
export declare const pythonicParser: ToolCallParser;
export declare const llama4PythonicParser: ToolCallParser;
export declare function createPythonicParser(): ToolCallParser;
export declare function createLlama4PythonicParser(): ToolCallParser;
//# sourceMappingURL=pythonic.d.ts.map