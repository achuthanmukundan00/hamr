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
import { randomUUID } from "node:crypto";
// Hermes / Qwen / ChatML:  <tool_call> { "name": ..., "arguments": ... } </tool_call>
const TOOL_CALL_TAG_RE = /<tool_call>\s*([\s\S]*?)\s*<\/tool_call>/gi;
// Functionary / Llama:  <function=NAME> { ...args... } </function>
const FUNCTION_TAG_RE = /<function\s*=\s*([^>\s]+)\s*>\s*([\s\S]*?)\s*<\/function>/gi;
/** Strip a leading/trailing ```json … ``` (or bare ```) fence if present. */
function stripCodeFence(s) {
    const trimmed = s.trim();
    const fence = /^```(?:json|tool_code)?\s*([\s\S]*?)\s*```$/i.exec(trimmed);
    return fence ? fence[1].trim() : trimmed;
}
/** Coerce an `arguments`/`parameters` field that may be an object or a JSON string. */
function coerceArgs(raw) {
    if (raw && typeof raw === "object" && !Array.isArray(raw)) {
        return raw;
    }
    if (typeof raw === "string") {
        try {
            const parsed = JSON.parse(raw);
            if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
                return parsed;
            }
        }
        catch {
            /* fall through */
        }
    }
    return {};
}
function isAllowed(name, knownToolNames) {
    if (!name)
        return false;
    // When the caller knows the tool set, only synthesize calls for real tools —
    // this avoids misreading prose that merely mentions `<tool_call>`.
    return !knownToolNames || knownToolNames.length === 0 || knownToolNames.includes(name);
}
function makeToolCall(name, args) {
    return { type: "toolCall", id: `call_${randomUUID().replace(/-/g, "").slice(0, 24)}`, name, arguments: args };
}
/** Parse the JSON body of a Hermes/Qwen `<tool_call>` block. */
function parseHermesBody(inner, knownToolNames) {
    const body = stripCodeFence(inner);
    if (!body)
        return null;
    let obj;
    try {
        obj = JSON.parse(body);
    }
    catch {
        return null;
    }
    if (!obj || typeof obj !== "object" || Array.isArray(obj))
        return null;
    const rec = obj;
    const name = typeof rec.name === "string" ? rec.name : typeof rec.tool === "string" ? rec.tool : "";
    if (!isAllowed(name, knownToolNames))
        return null;
    const args = coerceArgs(rec.arguments ?? rec.parameters ?? rec.args);
    return makeToolCall(name, args);
}
/**
 * Extract text-form tool calls from assistant output.
 *
 * @param text  The accumulated assistant text (or thinking) to scan.
 * @param knownToolNames  Optional allow-list of real tool names. When provided,
 *   only markup naming one of these tools is converted — preventing false
 *   positives from text that merely discusses tool-call syntax.
 */
export function extractTextToolCalls(text, knownToolNames) {
    if (!text || (!text.includes("<tool_call>") && !text.includes("<function"))) {
        return { toolCalls: [], cleanedText: text };
    }
    const toolCalls = [];
    let cleaned = text.replace(TOOL_CALL_TAG_RE, (match, inner) => {
        const tc = parseHermesBody(inner, knownToolNames);
        if (tc) {
            toolCalls.push(tc);
            return "";
        }
        return match; // leave unrecognized markup untouched
    });
    cleaned = cleaned.replace(FUNCTION_TAG_RE, (match, name, inner) => {
        const toolName = name.trim();
        if (!isAllowed(toolName, knownToolNames))
            return match;
        const tc = makeToolCall(toolName, coerceArgs(stripCodeFence(inner)));
        toolCalls.push(tc);
        return "";
    });
    return { toolCalls, cleanedText: cleaned.trim() };
}
//# sourceMappingURL=text-tool-calls.js.map