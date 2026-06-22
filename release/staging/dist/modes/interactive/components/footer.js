import { isAbsolute, relative, resolve, sep } from "node:path";
import { getSupportedThinkingLevels } from "@hamr/ai";
import { truncateToWidth, visibleWidth } from "@hamr/tui";
import { BUILT_IN_PROVIDER_DISPLAY_NAMES } from "../../../core/provider-display-names.js";
import { theme } from "../theme/theme.js";
/**
 * Sanitize text for display in a single-line status.
 * Removes newlines, tabs, carriage returns, and other control characters.
 */
function sanitizeStatusText(text) {
    return text
        .replace(/[\r\n\t]/g, " ")
        .replace(/ +/g, " ")
        .trim();
}
function formatTokens(count) {
    if (count < 1000)
        return count.toString();
    if (count < 10000)
        return `${(count / 1000).toFixed(1)}K`;
    if (count < 1000000)
        return `${Math.round(count / 1000)}K`;
    if (count < 10000000)
        return `${(count / 1000000).toFixed(1)}M`;
    return `${Math.round(count / 1000000)}M`;
}
function pct(used, total) {
    if (total <= 0)
        return "0%";
    return `${Math.round((used / total) * 100)}%`;
}
export function formatContextPart(tokens, contextWindow, percent, compact) {
    if (contextWindow <= 0)
        return undefined;
    if (tokens === null || tokens === undefined || percent === null || percent === undefined) {
        return compact ? `? / ${formatTokens(contextWindow)}` : `? used of ${formatTokens(contextWindow)} tokens`;
    }
    return compact
        ? `${pct(tokens, contextWindow)} / ${formatTokens(contextWindow)}`
        : `${pct(tokens, contextWindow)} used of ${formatTokens(contextWindow)} tokens`;
}
/**
 * Format the accumulated cost for the status bar. Uses 3-decimal precision so
 * sub-cent spend (e.g. $0.003) is visible rather than rounding to $0.00.
 * Shows accumulated cost when present. Omits the segment only when there is
 * no prior cloud spend AND the current model is zero-priced (relay/local).
 */
export function formatCostPart(totalCost, inputPricePerMillion, usingSubscription) {
    if (inputPricePerMillion <= 0 && totalCost <= 0)
        return undefined;
    return `$${totalCost.toFixed(3)}${usingSubscription ? " (sub)" : ""}`;
}
function hslToRgb(h, s, l) {
    const c = (1 - Math.abs(2 * l - 1)) * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = l - c / 2;
    let r = 0;
    let g = 0;
    let b = 0;
    if (h < 60) {
        r = c;
        g = x;
    }
    else if (h < 120) {
        r = x;
        g = c;
    }
    else if (h < 180) {
        g = c;
        b = x;
    }
    else if (h < 240) {
        g = x;
        b = c;
    }
    else if (h < 300) {
        r = x;
        b = c;
    }
    else {
        r = c;
        b = x;
    }
    return [Math.round((r + m) * 255), Math.round((g + m) * 255), Math.round((b + m) * 255)];
}
function providerDisplayName(provider) {
    if (provider === "relay")
        return "Relay";
    return BUILT_IN_PROVIDER_DISPLAY_NAMES[provider] ?? provider;
}
export function formatCwdForFooter(cwd, home) {
    if (!home)
        return cwd;
    const resolvedCwd = resolve(cwd);
    const resolvedHome = resolve(home);
    const relativeToHome = relative(resolvedHome, resolvedCwd);
    const isInsideHome = relativeToHome === "" ||
        (relativeToHome !== ".." && !relativeToHome.startsWith(`..${sep}`) && !isAbsolute(relativeToHome));
    if (!isInsideHome)
        return cwd;
    return relativeToHome === "" ? "~" : `~${sep}${relativeToHome}`;
}
/**
 * Footer component that shows Hamr's synax-style single-line status:
 * activity on the left, context/spend/tokens/provider/model/thinking on the right.
 */
export class FooterComponent {
    static { this.RAINBOW_LUT = (() => {
        const lightnessSteps = 16;
        const table = [];
        for (let h = 0; h < 360; h += 1) {
            const row = [];
            for (let l = 0; l < lightnessSteps; l += 1) {
                const [r, g, b] = hslToRgb(h, 0.9, 0.4 + (l / lightnessSteps) * 0.4);
                row.push(`\x1b[38;2;${r};${g};${b}m`);
            }
            table.push(row);
        }
        return table;
    })(); }
    constructor(session, footerData, requestRender) {
        this.session = session;
        this.footerData = footerData;
        this.requestRender = requestRender;
    }
    setSession(session) {
        this.session = session;
    }
    setAutoCompactEnabled(_enabled) { }
    invalidate() {
        // Rendered directly from session/provider state.
    }
    dispose() {
        this.stopAnimationTimer();
    }
    render(width) {
        if (this.isAnimating())
            this.startAnimationTimer();
        else
            this.stopAnimationTimer();
        const lines = [];
        if (width < 40) {
            lines.push(truncateToWidth(this.renderActivityText(), width, theme.fg("dim", "...")));
            return lines;
        }
        const left = this.renderActivityText();
        const right = this.renderRightSide(width);
        const leftWidth = visibleWidth(left);
        const rightForLine = right;
        const rightWidth = visibleWidth(rightForLine);
        // Not enough room for left + gap + right: show only right (truncated if needed)
        if (rightWidth + 2 >= width) {
            lines.push(truncateToWidth(rightForLine, width, theme.fg("dim", "...")));
        }
        else if (!right || leftWidth + rightWidth + 2 > width) {
            const availableLeft = width - rightWidth - 2;
            const trimmedLeft = truncateToWidth(left, Math.max(1, availableLeft), theme.fg("dim", "..."));
            const gap = width - visibleWidth(trimmedLeft) - rightWidth;
            lines.push(`${trimmedLeft}${" ".repeat(gap)}${rightForLine}`);
        }
        else {
            lines.push(`${left}${" ".repeat(width - leftWidth - rightWidth)}${rightForLine}`);
        }
        const extensionStatuses = this.footerData.getExtensionStatuses();
        if (extensionStatuses.size > 0) {
            const statusLine = Array.from(extensionStatuses.entries())
                .sort(([a], [b]) => a.localeCompare(b))
                .map(([, text]) => sanitizeStatusText(text))
                .join(" ");
            lines.push(truncateToWidth(statusLine, width, theme.fg("dim", "...")));
        }
        return lines;
    }
    isAnimating() {
        return this.session.isStreaming;
    }
    startAnimationTimer() {
        if (this.animationTimer || !this.requestRender)
            return;
        this.animationTimer = setInterval(() => {
            this.requestRender?.();
        }, 100);
    }
    stopAnimationTimer() {
        if (!this.animationTimer)
            return;
        clearInterval(this.animationTimer);
        this.animationTimer = undefined;
    }
    renderActivityText() {
        const text = this.session.isStreaming ? "Working..." : "Idle";
        if (!this.isAnimating())
            return theme.fg("dim", text);
        if (this.isMaxThinking()) {
            const t = Date.now() / 1000;
            const rainbow = FooterComponent.RAINBOW_LUT;
            const lightness = Math.floor(rainbow[0].length * 0.55);
            const hueSpread = 8;
            const hueOffset = Math.floor((t * 180) % 360);
            const hueStep = 360 / hueSpread;
            const parts = [];
            for (let i = 0; i < text.length; i += 1) {
                const hue = (360 - hueOffset + Math.floor(i * hueStep)) % 360;
                parts.push(rainbow[hue][lightness], text[i]);
            }
            return this.session.isStreaming ? theme.bold(`${parts.join("")}\x1b[0m`) : `${parts.join("")}\x1b[0m`;
        }
        const t = Date.now() / 1000;
        const shimmerSpeed = 14;
        const shimmerWidth = 4;
        const cycle = text.length + shimmerWidth;
        const phase = (t * shimmerSpeed) % cycle;
        let shimmered = "";
        for (let i = 0; i < text.length; i += 1) {
            const dist = Math.abs(i - phase);
            if (dist < shimmerWidth) {
                const brightness = 1 - dist / shimmerWidth;
                if (brightness > 0.66)
                    shimmered += theme.bold(text[i]);
                else if (brightness > 0.33)
                    shimmered += theme.fg("text", text[i]);
                else
                    shimmered += theme.fg("muted", text[i]);
            }
            else {
                shimmered += theme.fg("dim", text[i]);
            }
        }
        return this.session.isStreaming ? theme.bold(shimmered) : shimmered;
    }
    isMaxThinking() {
        const level = this.session.thinkingLevel || "off";
        const model = this.session.state.model;
        if (!model?.reasoning || level === "off")
            return false;
        const levels = getSupportedThinkingLevels(model).filter((entry) => entry !== "off");
        if (levels.length === 0)
            return true;
        return level === levels[levels.length - 1];
    }
    renderRightSide(width) {
        const state = this.session.state;
        const usage = this.getSessionUsage();
        const contextUsage = this.session.getContextUsage();
        const contextWindow = contextUsage?.contextWindow ?? state.model?.contextWindow ?? 0;
        const contextPercentValue = contextUsage?.percent;
        const compact = width < 100;
        const parts = [];
        // Context usage — always shown.
        const contextPercentDisplay = contextPercentValue !== null && contextPercentValue !== undefined
            ? `${contextPercentValue.toFixed(1)}%/${formatTokens(contextWindow)}`
            : `?/${formatTokens(contextWindow)}`;
        const coloredContext = (contextPercentValue ?? 0) > 90
            ? theme.fg("error", contextPercentDisplay)
            : (contextPercentValue ?? 0) > 70
                ? theme.fg("warning", contextPercentDisplay)
                : theme.fg("dim", contextPercentDisplay);
        parts.push(coloredContext);
        // Cost — show whenever there's accumulated spend or an active OAuth subscription.
        const usingSubscription = state.model ? this.session.modelRegistry.isUsingOAuth(state.model) : false;
        if (usage.totalCost > 0 || usingSubscription) {
            parts.push(theme.fg("dim", `$${usage.totalCost.toFixed(3)}${usingSubscription ? " (sub)" : ""}`));
        }
        // Token counts — compact: ↑10k ↓5k
        if (usage.totalInput > 0 || usage.totalOutput > 0) {
            const tokens = [];
            if (usage.totalInput > 0)
                tokens.push(`↑${formatTokens(usage.totalInput)}`);
            if (usage.totalOutput > 0)
                tokens.push(`↓${formatTokens(usage.totalOutput)}`);
            parts.push(theme.fg("dim", tokens.join(" ")));
        }
        // Cache hit rate — always show when cache has been used
        if ((usage.totalCacheRead > 0 || usage.totalCacheWrite > 0) && usage.latestCacheHitRate !== undefined) {
            parts.push(theme.fg("dim", `CH${usage.latestCacheHitRate.toFixed(1)}%`));
        }
        // Model — compact: provider/ glyph name thinking
        if (state.model) {
            const providerTag = compact
                ? theme.fg("muted", `${state.model.provider}/`)
                : theme.fg("muted", `(${providerDisplayName(state.model.provider)}) `);
            const glyph = theme.modelGlyph(state.model.provider, state.model.name ?? state.model.id);
            const modelName = state.model.name || state.model.id;
            const modelBrandAnsi = theme.modelColor(state.model.provider, modelName);
            const model = `${modelBrandAnsi}${glyph} ${modelName}\x1b[39m`;
            const thinking = state.model.reasoning ? theme.fg("dim", ` ${this.session.thinkingLevel || "off"}`) : "";
            parts.push(`${providerTag}${model}${thinking}`);
        }
        else {
            parts.push(theme.fg("dim", "no-model"));
        }
        return parts.join("  ");
    }
    getSessionUsage() {
        let totalInput = 0;
        let totalOutput = 0;
        let totalCacheRead = 0;
        let totalCacheWrite = 0;
        let totalCost = 0;
        let latestCacheHitRate;
        for (const entry of this.session.sessionManager.getEntries()) {
            if (entry.type !== "message" || entry.message.role !== "assistant")
                continue;
            const { usage } = entry.message;
            totalInput += usage.input;
            totalOutput += usage.output;
            totalCacheRead += usage.cacheRead;
            totalCacheWrite += usage.cacheWrite;
            totalCost += usage.cost.total;
            const latestPromptTokens = usage.input + usage.cacheRead + usage.cacheWrite;
            latestCacheHitRate = latestPromptTokens > 0 ? (usage.cacheRead / latestPromptTokens) * 100 : undefined;
        }
        return { totalInput, totalOutput, totalCacheRead, totalCacheWrite, totalCost, latestCacheHitRate };
    }
}
//# sourceMappingURL=footer.js.map