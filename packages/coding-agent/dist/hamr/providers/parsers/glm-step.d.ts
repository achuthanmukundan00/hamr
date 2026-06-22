/**
 * GLM and Step tool-call parsers.
 *
 * GLM 4.5/4.7 models and Step 3/3.5 models use specialized tool-call formats.
 *
 * GLM format (as documented by vLLM):
 *   Uses a function-call token followed by a JSON object:
 *   <|tool_call|>{"name": "get_weather", "arguments": {"location": "SF"}}
 *
 * Step 3 format:
 *   Uses XML-style tags similar to Qwen but with Step-specific markup.
 *
 * These parsers are currently stubs that fall back to Hermes-style parsing
 * and will be refined as format-specific documentation becomes available.
 *
 * Reference: vLLM
 *   --tool-call-parser glm45, glm47, step3, step3p5
 */
import type { ToolCallParser } from "./types.ts";
export declare const glm45Parser: ToolCallParser;
export declare const glm47Parser: ToolCallParser;
export declare const step3Parser: ToolCallParser;
export declare const step3p5Parser: ToolCallParser;
export declare function createGlm45Parser(): ToolCallParser;
export declare function createGlm47Parser(): ToolCallParser;
export declare function createStep3Parser(): ToolCallParser;
export declare function createStep3p5Parser(): ToolCallParser;
//# sourceMappingURL=glm-step.d.ts.map