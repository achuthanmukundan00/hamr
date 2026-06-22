import { Text } from "@hamr/tui";
// ─── Token estimation ─────────────────────────────────────────────────────────
function estimateChars(text) {
    return Math.ceil(text.length / 4);
}
function computeBreakdown(ctx) {
    const model = ctx.model;
    const contextUsage = ctx.getContextUsage();
    const contextWindow = contextUsage?.contextWindow ?? model?.contextWindow ?? 0;
    const tokens = contextUsage?.tokens ?? null;
    const percent = contextUsage?.percent ?? null;
    const opts = ctx.getSystemPromptOptions();
    const systemPromptText = ctx.getSystemPrompt();
    // Estimate each system-prompt section.
    // Skills are no longer in the system prompt (loaded on demand).
    const skillsTokens = 0;
    const contextFilesText = (opts.contextFiles ?? [])
        .map(({ path, content }) => `<project_instructions path="${path}">\n${content}\n</project_instructions>\n\n`)
        .join("");
    const contextFilesTokens = estimateChars(contextFilesText);
    const totalSystemTokens = estimateChars(systemPromptText);
    const baseSystemTokens = Math.max(0, totalSystemTokens - contextFilesTokens);
    const messagesAndTools = tokens !== null ? Math.max(0, tokens - totalSystemTokens) : null;
    return {
        modelName: model?.name,
        modelId: model?.id,
        contextWindow,
        tokens,
        percent,
        systemPrompt: baseSystemTokens,
        skills: skillsTokens,
        contextFiles: contextFilesTokens,
        messagesAndTools,
    };
}
// ─── Rendering ───────────────────────────────────────────────────────────────
function fmt(n) {
    if (n < 1000)
        return String(Math.round(n));
    if (n < 1_000_000)
        return `${(n / 1000).toFixed(1)}k`;
    return `${(n / 1_000_000).toFixed(1)}m`;
}
function renderDisplay(breakdown, theme) {
    const { contextWindow, tokens, percent } = breakdown;
    // 25 slots at 4k tokens each = meaningful fill at any context size.
    const SLOTS = 25;
    const TOKENS_PER_SLOT = 4000;
    const filledSlots = tokens !== null ? Math.round(tokens / TOKENS_PER_SLOT) : 0;
    const cappedFilled = Math.min(filledSlots, SLOTS);
    const showOverflow = filledSlots > SLOTS;
    const usageColor = (percent ?? 0) > 90 ? "error" : (percent ?? 0) > 70 ? "warning" : "accent";
    const iconRows = [];
    for (let row = 0; row < 5; row++) {
        const icons = [];
        for (let col = 0; col < 5; col++) {
            const slot = row * 5 + col;
            if (slot < cappedFilled) {
                icons.push(theme.fg(usageColor, "⛁"));
            }
            else if (slot === cappedFilled && showOverflow) {
                icons.push(theme.fg(usageColor, "+"));
            }
            else {
                icons.push(theme.fg("dim", "⛶"));
            }
        }
        iconRows.push(icons.join(" "));
    }
    const tokenStr = tokens !== null && contextWindow > 0
        ? `${fmt(tokens)}/${fmt(contextWindow)} tokens`
        : tokens !== null
            ? `${fmt(tokens)}/0 tokens`
            : contextWindow > 0
                ? `?/${fmt(contextWindow)} tokens`
                : "? / ? tokens";
    const pctStr = percent !== null ? ` (${Math.round(percent)}%)` : "";
    const freeTokens = tokens !== null && contextWindow > 0 ? contextWindow - tokens : null;
    const freePct = freeTokens !== null && contextWindow > 0 ? (freeTokens / contextWindow) * 100 : null;
    const INDENT = "            ";
    const dot = (color, ch) => theme.fg(color, ch);
    const bold = (s) => theme.bold(s);
    const lines = [];
    lines.push(theme.bold("Context Usage"));
    lines.push("");
    lines.push(`${iconRows[0]}    ${breakdown.modelName ?? "No model"}`);
    lines.push(`${iconRows[1]}    ${theme.fg("dim", breakdown.modelId ?? "")}`);
    lines.push(`${iconRows[2]}    ${theme.fg("dim", `${tokenStr}${pctStr}`)}`);
    lines.push(iconRows[3]);
    lines.push(`${iconRows[4]}    ${theme.fg("dim", theme.italic("Estimated usage by category"))}`);
    lines.push("");
    lines.push(`${INDENT}${dot("dim", "⛁")} ${bold("System prompt:")} ${fmt(breakdown.systemPrompt)} tokens`);
    if (breakdown.skills > 0) {
        lines.push(`${INDENT}${dot("accent", "⛁")} ${bold("Skills:")} ${fmt(breakdown.skills)} tokens`);
    }
    if (breakdown.contextFiles > 0) {
        lines.push(`${INDENT}${dot("warning", "⛁")} ${bold("Context files:")} ${fmt(breakdown.contextFiles)} tokens`);
    }
    lines.push(`${INDENT}${dot(usageColor, "⛁")} ${bold("Messages + tools:")} ${breakdown.messagesAndTools !== null ? `${fmt(breakdown.messagesAndTools)} tokens` : "unknown"}`);
    lines.push(`${INDENT}${dot("dim", "⛶")} ${bold("Free space:")} ${freeTokens !== null && contextWindow > 0 ? `${fmt(freeTokens)} (${Math.round(freePct)}%)` : "unknown"}`);
    return lines.join("\n");
}
// ─── Extension ───────────────────────────────────────────────────────────────
export const hamrContextExtension = (pi) => {
    pi.registerMessageRenderer("hamr.context", (message, _options, theme) => {
        if (!message.details)
            return undefined;
        return new Text(renderDisplay(message.details, theme), 1, 0);
    });
    pi.registerCommand("context", {
        description: "Show context window usage",
        handler: async (_args, ctx) => {
            const breakdown = computeBreakdown(ctx);
            pi.sendMessage({
                customType: "hamr.context",
                content: `Context: ${breakdown.tokens !== null ? fmt(breakdown.tokens) : "?"}/${fmt(breakdown.contextWindow)} tokens`,
                display: true,
                details: breakdown,
            });
        },
    });
};
//# sourceMappingURL=context.js.map