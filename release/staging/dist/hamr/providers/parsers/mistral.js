/**
 * Mistral tool-call parser.
 *
 * Parses Mistral-format tool calls:
 *
 *   [TOOL_CALLS][{"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}]
 *
 * The model outputs a [TOOL_CALLS] prefix followed by a JSON array of
 * tool call objects. Each object has "name" and "arguments" fields.
 * Multiple calls can appear in the same JSON array.
 *
 * Variants:
 *   - mistral: standard Mistral format
 *
 * Reference: vLLM docs/features/tool_calling.md
 *   --tool-call-parser mistral
 *   vllm/entrypoints/openai/tool_parsers/mistral_tool_parser.py
 */
import { generateCallId, safeJsonParse, sanitizeReasoningTags } from "./utils.js";
const PARSER_ID = "mistral";
const DESCRIPTION = 'Mistral format: [TOOL_CALLS][{"name":"...","arguments":{...}}, ...]';
const FAMILIES = ["Mistral", "Mixtral", "Mistral Nemo", "Codestral"];
const TOOL_CALLS_PREFIX = "[TOOL_CALLS]";
export const mistralParser = {
    id: PARSER_ID,
    description: DESCRIPTION,
    modelFamilies: FAMILIES,
    parse(content) {
        const sanitized = sanitizeReasoningTags(content);
        const prefixIdx = sanitized.indexOf(TOOL_CALLS_PREFIX);
        if (prefixIdx === -1) {
            return {
                ok: true,
                parserId: PARSER_ID,
                calls: [],
                content: sanitized,
            };
        }
        const before = sanitized.slice(0, prefixIdx).trim();
        const afterPrefix = sanitized.slice(prefixIdx + TOOL_CALLS_PREFIX.length).trim();
        if (!afterPrefix.startsWith("[")) {
            return {
                ok: false,
                parserId: PARSER_ID,
                calls: [],
                content: sanitized,
                error: "Mistral: [TOOL_CALLS] not followed by JSON array",
            };
        }
        // Find matching closing bracket
        const closeIdx = findBalancedClose(afterPrefix, "[", "]");
        const jsonStr = closeIdx !== -1 ? afterPrefix.slice(0, closeIdx + 1) : afterPrefix;
        const afterContent = closeIdx !== -1 ? afterPrefix.slice(closeIdx + 1).trim() : "";
        const parsed = safeJsonParse(jsonStr);
        if (!parsed.ok) {
            return {
                ok: false,
                parserId: PARSER_ID,
                calls: [],
                content: sanitized,
                error: `Mistral: ${parsed.error}`,
            };
        }
        if (!Array.isArray(parsed.value)) {
            return {
                ok: false,
                parserId: PARSER_ID,
                calls: [],
                content: sanitized,
                error: "Mistral: [TOOL_CALLS] content is not a JSON array",
            };
        }
        const calls = [];
        const warnings = [];
        for (let i = 0; i < parsed.value.length; i++) {
            const item = parsed.value[i];
            if (typeof item !== "object" || item === null) {
                warnings.push(`Mistral item ${i}: not a JSON object, skipping`);
                continue;
            }
            const obj = item;
            const name = obj.name ?? obj.tool_name ?? obj.function;
            if (typeof name !== "string" || !name.trim()) {
                warnings.push(`Mistral item ${i}: missing "name", skipping`);
                continue;
            }
            let args = {};
            const rawArgs = obj.arguments ?? obj.parameters ?? obj.input;
            if (typeof rawArgs === "string") {
                const parsedArgs = safeJsonParse(rawArgs);
                if (parsedArgs.ok &&
                    typeof parsedArgs.value === "object" &&
                    parsedArgs.value !== null &&
                    !Array.isArray(parsedArgs.value)) {
                    args = parsedArgs.value;
                }
                else {
                    warnings.push(`Mistral item ${i}: arguments string parse failed`);
                }
            }
            else if (typeof rawArgs === "object" && rawArgs !== null && !Array.isArray(rawArgs)) {
                args = rawArgs;
            }
            calls.push({
                id: generateCallId((obj.id ?? obj.call_id), i + 1),
                name: name.trim(),
                arguments: args,
                rawSource: JSON.stringify(item),
                parserId: PARSER_ID,
            });
        }
        const nonToolContent = [before, afterContent].filter(Boolean).join("\n");
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
function findBalancedClose(text, open, close) {
    let depth = 0;
    let inString = false;
    let escape = false;
    for (let i = 0; i < text.length; i++) {
        const ch = text[i];
        if (escape) {
            escape = false;
            continue;
        }
        if (ch === "\\" && inString) {
            escape = true;
            continue;
        }
        if (ch === '"') {
            inString = !inString;
            continue;
        }
        if (inString)
            continue;
        if (ch === open)
            depth++;
        else if (ch === close) {
            depth--;
            if (depth === 0)
                return i;
        }
    }
    return -1;
}
// ─── Factory ──────────────────────────────────────────────
export function createMistralParser() {
    return mistralParser;
}
//# sourceMappingURL=mistral.js.map