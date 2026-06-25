/**
 * Generic tool-call parser (fallback).
 *
 * Handles both JSON and XML tool-call formats so unrecognised model families
 * (Apodex, Qwen derivatives, …) are not silently broken.
 *
 * Strategies (tried in order):
 * 1. <tool_call> blocks → JSON first (Hermes-style), then XML (Qwen3-style)
 * 2. ```json fenced code blocks
 * 3. Bare JSON objects (last resort)
 */
import { toolCallParserRegistry } from "./registry.js";
import { extractDelimitedBlocks, generateCallId, safeJsonParse, sanitizeReasoningTags } from "./utils.js";
const PARSER_ID = "generic";
const DESCRIPTION = "Generic multi-strategy fallback: handles JSON (Hermes-style) and XML (Qwen3-style) tool calls";
const FAMILIES = ["Any", "Unknown", "Generic"];
export const genericParser = {
    id: PARSER_ID,
    description: DESCRIPTION,
    modelFamilies: FAMILIES,
    parse(content) {
        const sanitized = sanitizeReasoningTags(content);
        const calls = [];
        const warnings = [];
        // Strategy 1: <tool_call>...</tool_call> blocks
        const toolCallBlocks = extractDelimitedBlocks(sanitized, "<tool_call>", "</tool_call>");
        let sawMalformedBlock = false;
        for (const block of toolCallBlocks.blocks) {
            const trimmed = block.trim();
            if (!trimmed)
                continue;
            const parsed = safeJsonParse(trimmed);
            if (!parsed.ok) {
                sawMalformedBlock = true;
                continue;
            }
            if (typeof parsed.value !== "object" || parsed.value === null) {
                sawMalformedBlock = true;
                continue;
            }
            const obj = parsed.value;
            // Check for array of tool_calls (OpenAI-style wrapped)
            if (Array.isArray(obj.tool_calls)) {
                for (const tc of obj.tool_calls) {
                    if (tc && typeof tc === "object") {
                        const tcObj = tc;
                        const fn = tcObj.function;
                        if (fn && typeof fn === "object") {
                            const fnObj = fn;
                            const name = fnObj.name;
                            if (typeof name === "string") {
                                const args = parseArgsValue(fnObj.arguments);
                                calls.push({
                                    id: generateCallId((tcObj.id ?? fnObj.id), calls.length + 1),
                                    name,
                                    arguments: args,
                                    rawSource: JSON.stringify(tc),
                                    parserId: PARSER_ID,
                                });
                            }
                        }
                    }
                }
                continue;
            }
            // Single named call
            const name = obj.name ?? obj.tool_name;
            if (typeof name !== "string" || !name.trim()) {
                sawMalformedBlock = true;
                continue;
            }
            const args = parseArgsValue(obj.arguments ?? obj.parameters ?? obj.input);
            calls.push({
                id: generateCallId((obj.id ?? obj.call_id), calls.length + 1),
                name: name.trim(),
                arguments: args,
                rawSource: trimmed,
                parserId: PARSER_ID,
            });
        }
        // Strategy 2: ```json fenced code blocks
        const fencedBlocks = extractFencedJsonBlocks(sanitized);
        for (const block of fencedBlocks) {
            const parsed = safeJsonParse(block);
            if (!parsed.ok)
                continue;
            if (typeof parsed.value !== "object" || parsed.value === null)
                continue;
            const obj = parsed.value;
            // Handle OpenAI-style tool_calls array inside the block
            if (Array.isArray(obj.tool_calls)) {
                for (const tc of obj.tool_calls) {
                    if (tc && typeof tc === "object") {
                        const tcObj = tc;
                        const fn = tcObj.function;
                        if (fn && typeof fn === "object") {
                            const fnObj = fn;
                            const name = fnObj.name;
                            if (typeof name === "string") {
                                const args = parseArgsValue(fnObj.arguments);
                                calls.push({
                                    id: generateCallId((tcObj.id ?? fnObj.id), calls.length + 1),
                                    name,
                                    arguments: args,
                                    rawSource: JSON.stringify(tc),
                                    parserId: PARSER_ID,
                                    warnings: ["parsed from fenced code block"],
                                });
                            }
                        }
                    }
                }
                continue;
            }
            // Handle single named call in fenced block
            const name = obj.name ?? obj.tool_name;
            if (typeof name !== "string" || !name.trim())
                continue;
            const args = parseArgsValue(obj.arguments ?? obj.parameters ?? obj.input);
            calls.push({
                id: generateCallId((obj.id ?? obj.call_id), calls.length + 1),
                name: name.trim(),
                arguments: args,
                rawSource: block,
                parserId: PARSER_ID,
                warnings: ["parsed from fenced code block"],
            });
        }
        // Strategy 3: Bare JSON object (last resort, only when no other calls found)
        if (calls.length === 0) {
            const trimmed = sanitized.trim();
            if (trimmed.startsWith("{")) {
                const parsed = safeJsonParse(trimmed);
                if (parsed.ok && typeof parsed.value === "object" && parsed.value !== null) {
                    const obj = parsed.value;
                    // Handle OpenAI-style tool_calls array
                    if (Array.isArray(obj.tool_calls)) {
                        for (const tc of obj.tool_calls) {
                            if (tc && typeof tc === "object") {
                                const tcObj = tc;
                                const fn = tcObj.function;
                                if (fn && typeof fn === "object") {
                                    const fnObj = fn;
                                    const name = fnObj.name;
                                    if (typeof name === "string") {
                                        const args = parseArgsValue(fnObj.arguments);
                                        calls.push({
                                            id: generateCallId((tcObj.id ?? fnObj.id), calls.length + 1),
                                            name,
                                            arguments: args,
                                            rawSource: JSON.stringify(tc),
                                            parserId: PARSER_ID,
                                            warnings: ["parsed from bare JSON text"],
                                        });
                                    }
                                }
                            }
                        }
                    }
                    else {
                        const name = obj.name ?? obj.tool_name;
                        if (typeof name === "string" && name.trim()) {
                            const args = parseArgsValue(obj.arguments ?? obj.parameters ?? obj.input);
                            calls.push({
                                id: generateCallId((obj.id ?? obj.call_id), 1),
                                name: name.trim(),
                                arguments: args,
                                rawSource: trimmed,
                                parserId: PARSER_ID,
                                warnings: ["parsed from bare JSON text"],
                            });
                        }
                    }
                }
            }
        }
        // Strategy 1b: If <tool_call> blocks contained non-JSON content,
        // try the Qwen3 XML parser as a fallback.  This catches Qwen-family
        // models (including unrecognised derivatives like Apodex) that emit
        // <function=…><parameter=…>…</parameter></function> XML.
        // Guarded by isStandaloneToolCallContent so example blocks inside
        // fenced-code prose aren't parsed as real tool calls.
        let xmlNonToolContent;
        if (sawMalformedBlock &&
            calls.length === 0 &&
            toolCallBlocks.blocks.length > 0 &&
            isStandaloneToolCallContent(sanitized, toolCallBlocks)) {
            const xmlResult = toolCallParserRegistry.parse("qwen3_xml", sanitized);
            if (xmlResult.ok && xmlResult.calls.length > 0) {
                for (const call of xmlResult.calls) {
                    call.parserId = PARSER_ID;
                    call.warnings = [...(call.warnings ?? []), "parsed via qwen3_xml fallback in generic parser"];
                }
                calls.push(...xmlResult.calls);
                sawMalformedBlock = false;
                xmlNonToolContent = xmlResult.content || undefined;
            }
            else {
                // Neither JSON nor XML could parse the blocks and they look
                // intentional → signal failure so the repair cascade can try.
                return {
                    ok: false,
                    parserId: PARSER_ID,
                    calls: [],
                    content: sanitized,
                    error: "tool_call block contained malformed content (not valid JSON or Qwen XML)",
                };
            }
        }
        // Compute non-tool content: prefer the XML parser's extraction
        // (which understands <function=…> blocks), then fall back to the
        // generic delimiter-based approach.
        let nonToolContent;
        if (xmlNonToolContent) {
            nonToolContent = xmlNonToolContent;
        }
        else if (sawMalformedBlock === false && calls.length > 0 && toolCallBlocks.blocks.length > 0) {
            nonToolContent = [toolCallBlocks.before, ...toolCallBlocks.between, toolCallBlocks.after]
                .filter(Boolean)
                .join("\n")
                .trim();
        }
        else {
            // Blocks couldn't be parsed (not JSON, not Qwen XML), and they're
            // interleaved with prose — strip blocks so the model sees clean content.
            nonToolContent = [toolCallBlocks.before, ...toolCallBlocks.between, toolCallBlocks.after]
                .filter(Boolean)
                .join("\n")
                .trim();
        }
        return {
            ok: true,
            parserId: PARSER_ID,
            calls,
            content: nonToolContent,
            warnings: warnings.length > 0 ? warnings : undefined,
        };
    },
};
// ─── Helpers ──────────────────────────────────────────────
function parseArgsValue(raw) {
    if (typeof raw === "string") {
        const parsed = safeJsonParse(raw);
        if (parsed.ok && typeof parsed.value === "object" && parsed.value !== null && !Array.isArray(parsed.value)) {
            return parsed.value;
        }
    }
    else if (typeof raw === "object" && raw !== null && !Array.isArray(raw)) {
        return raw;
    }
    return {};
}
function extractFencedJsonBlocks(content) {
    return [...content.matchAll(/```(?:json)?\s*([\s\S]*?)\s*```/g)].map((m) => m[1]);
}
/**
 * Check if the content is primarily a tool-call response (standalone blocks).
 * If tool_calls appear inside prose/fenced code, they are likely examples.
 */
function isStandaloneToolCallContent(content, blocks) {
    // If content is entirely within fenced code blocks, it's not a standalone tool call
    const fencedContent = content.match(/```[\s\S]*?```/g);
    if (fencedContent) {
        for (const fenced of fencedContent) {
            if (fenced.includes("<tool_call>"))
                return false;
        }
    }
    // If there's substantial non-whitespace content before the first block, not standalone
    if (blocks.before.trim().length > 0) {
        // Allow a short preamble (like model intro text) but not full prose
        if (blocks.before.trim().split(/\s+/).length > 5)
            return false;
    }
    // If there's text between blocks that looks like prose, not standalone
    for (const between of blocks.between) {
        if (between.trim().split(/\s+/).length > 5)
            return false;
    }
    // If there's substantial text after the last block, not standalone
    if (blocks.after.trim().split(/\s+/).length > 5)
        return false;
    return true;
}
// ─── Factory ──────────────────────────────────────────────
export function createGenericParser() {
    return genericParser;
}
//# sourceMappingURL=generic.js.map