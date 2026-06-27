import { Text } from "@hamr/tui";
// ─── Token estimation ─────────────────────────────────────────────────────────
/**
 * Rough byte-count token estimate.
 *
 * Uses a 3.0 bytes-per-token ratio (round, not ceil) because modern
 * tokenizers (Claude, GPT-4o, DeepSeek V4) encode English text at
 * roughly 2.5–3.5 chars per token.  4.0 overestimates by ~33%.
 *
 * For JSON / tool-schema content the ratio is closer to 2.0 (many
 * single-char tokens: `{`, `}`, `"`, `,`), so callers may override.
 */
function estimateTokens(text, bytesPerToken = 3.0) {
    if (!text)
        return 0;
    return Math.round(text.length / bytesPerToken);
}
function computeBreakdown(ctx) {
    const model = ctx.model;
    const contextUsage = ctx.getContextUsage();
    const contextWindow = contextUsage?.contextWindow ?? model?.contextWindow ?? 0;
    const tokens = contextUsage?.tokens ?? null;
    const percent = contextUsage?.percent ?? null;
    const opts = ctx.getSystemPromptOptions();
    const systemPromptText = ctx.getSystemPrompt();
    // ── System prompt sections ──────────────────────────────────────────
    // Reconstruct each section the same way buildSystemPrompt() does, so
    // we can count them individually.
    const cats = [];
    // 1. Base instructions (everything before <project_context>, <available_skills>,
    //    "Current date:", and "Current working directory:").
    const skillsIdx = systemPromptText.indexOf("<available_skills>");
    const projectCtxIdx = systemPromptText.indexOf("<project_context>");
    const dateIdx = systemPromptText.lastIndexOf("Current date:");
    // The base slice goes from start up to the earlier of skills / project-context.
    // (Date + cwd footer is included in system prompt — not worth its own category.)
    let baseEnd = systemPromptText.length;
    for (const idx of [skillsIdx, projectCtxIdx]) {
        if (idx > 0 && idx < baseEnd)
            baseEnd = idx;
    }
    const baseText = systemPromptText.slice(0, baseEnd).trim();
    if (baseText) {
        cats.push({ name: "System prompt", tokens: estimateTokens(baseText, 3.0), color: "text" });
    }
    // 2. Skills (from <available_skills> block)
    if (skillsIdx > 0) {
        let skillsEnd = systemPromptText.length;
        const afterSkills = systemPromptText.indexOf("</available_skills>", skillsIdx);
        if (afterSkills > 0)
            skillsEnd = afterSkills + "</available_skills>".length;
        const skillsText = systemPromptText.slice(skillsIdx, skillsEnd);
        if (skillsText) {
            // Skills are XML — token-inefficient. Use 2.5 bytes/token.
            cats.push({ name: "Skills", tokens: estimateTokens(skillsText, 2.5), color: "warning" });
        }
    }
    // 3. Context files (from <project_context> block)
    if (projectCtxIdx > 0) {
        let ctxEnd = systemPromptText.length;
        const afterCtx = systemPromptText.indexOf("</project_context>", projectCtxIdx);
        if (afterCtx > 0)
            ctxEnd = afterCtx + "</project_context>".length;
        const ctxText = systemPromptText.slice(projectCtxIdx, ctxEnd);
        if (ctxText) {
            cats.push({ name: "Context files", tokens: estimateTokens(ctxText), color: "accent" });
        }
    }
    // ── Messages ────────────────────────────────────────────────────────
    // The contextUsage.tokens already includes the system prompt.  If the
    // API reported real usage we trust its total; otherwise we estimate
    // the whole thing from chars.
    const totalSystemTokens = cats.reduce((sum, c) => sum + c.tokens, 0);
    // Check whether we have a real API-based token count vs a pure estimate.
    const hasApiUsage = tokens !== null && tokens > 0 && tokens > totalSystemTokens * 0.5;
    let messagesTokens = null;
    if (tokens !== null) {
        messagesTokens = Math.max(0, tokens - totalSystemTokens);
    }
    if (messagesTokens !== null && messagesTokens > 0) {
        cats.push({ name: "Messages + tools", tokens: messagesTokens, color: "accent" });
    }
    return {
        modelName: model?.name,
        modelId: model?.id,
        contextWindow,
        tokens,
        percent,
        categories: cats,
        fromApi: hasApiUsage,
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
    const sourceNote = breakdown.fromApi
        ? theme.fg("dim", theme.italic("Usage from last API response"))
        : theme.fg("dim", theme.italic("Estimated usage (no API response yet)"));
    lines.push(`${iconRows[4]}    ${sourceNote}`);
    lines.push("");
    for (const cat of breakdown.categories) {
        const prefix = breakdown.fromApi ? "" : "~";
        lines.push(`${INDENT}${dot(cat.color, "⛁")} ${bold(`${cat.name}:`)} ${prefix}${fmt(cat.tokens)} tokens`);
    }
    lines.push("");
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