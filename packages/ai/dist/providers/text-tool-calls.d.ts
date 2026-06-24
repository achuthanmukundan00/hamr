/**
 * Fallback parser for tool calls emitted as *text* rather than structured
 * `tool_calls` deltas.
 *
 * Many local models served over OpenAI-compatible endpoints (llama.cpp, vLLM,
 * Ollama, …) don't emit OpenAI's structured `tool_calls`. Instead the chat
 * template bakes the tool call into the assistant's text/thinking channel using
 * the model's native markup — most commonly the Hermes/Qwen `<tool_call>…JSON…
 * </tool_call>` form, or the functionary/llama `<function=name>…JSON…</function>`
 * form. When that markup lands in `content` as plain text, the harness never
 * turns it into a `toolCall` block, the turn finishes with `finish_reason:
 * "stop"`, and the agent goes idle mid-task (needing a manual "continue").
 *
 * This module recognizes those text formats and converts them into real
 * {@link ToolCall} blocks so the agent loop executes them like native calls.
 */
import type { ToolCall } from "../types.ts";
export interface TextToolCallExtraction {
    /** Tool calls recovered from the text, in document order. */
    toolCalls: ToolCall[];
    /** The input text with recognized tool-call markup removed. */
    cleanedText: string;
}
/**
 * Extract text-form tool calls from assistant output.
 *
 * @param text  The accumulated assistant text (or thinking) to scan.
 * @param knownToolNames  Optional allow-list of real tool names. When provided,
 *   only markup naming one of these tools is converted — preventing false
 *   positives from text that merely discusses tool-call syntax.
 */
export declare function extractTextToolCalls(text: string, knownToolNames?: ReadonlyArray<string>): TextToolCallExtraction;
//# sourceMappingURL=text-tool-calls.d.ts.map