import { getAssistantText, getThinkingText, hasToolCalls, modelKey } from "./helpers.js";
import { detectParserId } from "./providers/parsers/types.js";
import { parseModelOutput } from "./providers/tool-calls.js";
import { buildHamrProviderRegistrations } from "./startup-config.js";
export const parserByModel = new Map();
function parserFor(message, ctx) {
    return (parserByModel.get(modelKey(message.provider, message.model)) ??
        (ctx.model ? parserByModel.get(modelKey(ctx.model.provider, ctx.model.id)) : undefined) ??
        detectParserId(message.model) ??
        (ctx.model ? detectParserId(ctx.model.id) : undefined) ??
        "generic");
}
export function repairLocalToolCalls(message, ctx) {
    if (hasToolCalls(message))
        return undefined;
    const text = getAssistantText(message);
    const thinking = getThinkingText(message);
    if (!text.trim() && !thinking?.trim())
        return undefined;
    const parsed = parseModelOutput(text, parserFor(message, ctx), thinking);
    if (parsed.toolCalls.length === 0)
        return undefined;
    const content = [];
    if (parsed.reasoning) {
        content.push({ type: "thinking", thinking: parsed.reasoning });
    }
    if (parsed.assistantText.trim()) {
        content.push({ type: "text", text: parsed.assistantText.trim() });
    }
    for (const call of parsed.toolCalls) {
        content.push({
            type: "toolCall",
            id: call.id,
            name: call.name,
            arguments: call.arguments,
        });
    }
    return {
        ...message,
        content,
        stopReason: "toolUse",
        diagnostics: [
            ...(message.diagnostics ?? []),
            ...parsed.warnings.map((warning) => ({
                type: "hamr.tool_call_repair",
                timestamp: Date.now(),
                details: {
                    source: warning.source,
                    message: warning.message,
                },
            })),
        ],
    };
}
export function hasSubstantialContent(event) {
    switch (event.type) {
        case "start":
        case "done":
        case "error":
            return false;
        case "text_delta":
        case "thinking_delta":
        case "toolcall_delta":
            return event.delta.trim().length > 0;
        case "text_start":
        case "thinking_start":
        case "toolcall_start":
        case "text_end":
        case "thinking_end":
        case "toolcall_end":
            return true;
        default:
            return false;
    }
}
export async function registerHamrProviders(pi, config) {
    const registrations = await buildHamrProviderRegistrations(config);
    for (const registration of registrations) {
        for (const [modelId, parserId] of registration.parserByModel) {
            parserByModel.set(modelKey(registration.name, modelId), parserId);
        }
        pi.registerProvider(registration.name, registration.config);
    }
}
//# sourceMappingURL=repair.js.map