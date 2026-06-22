/**
 * Llama 3 JSON tool-call parser.
 *
 * Parses Llama 3.x JSON-format tool calls:
 *
 *   <|python_tag|>{"name": "get_weather", "parameters": {"location": "SF", "unit": "celsius"}}
 *
 * The model outputs a `<|python_tag|>` prefix followed by a JSON object
 * with "name" and "parameters" fields. Multiple calls can appear as
 * separate `<|python_tag|>` blocks.
 *
 * vLLM also supports a custom chat template that wraps calls in a
 * `<|start_header_id|>assistant<|end_header_id|>` structure, but
 * the parser handles the raw `<|python_tag|>` blocks directly.
 *
 * Reference: vLLM docs/features/tool_calling.md
 *   --tool-call-parser llama3_json
 *   --chat-template examples/tool_chat_template_llama3.1_json.jinja
 *   vllm/entrypoints/openai/tool_parsers/llama_tool_parser.py
 */
import type { ToolCallParser } from "./types.ts";
export declare const llama3JsonParser: ToolCallParser;
export declare function createLlama3JsonParser(): ToolCallParser;
//# sourceMappingURL=llama3-json.d.ts.map