import { Box, Container, Markdown, Spacer, Text } from "@hamr/tui";
import { getMarkdownTheme, theme } from "../theme/theme.js";
const OSC133_ZONE_START = "\x1b]133;A\x07";
const OSC133_ZONE_END = "\x1b]133;B\x07";
const OSC133_ZONE_FINAL = "\x1b]133;C\x07";
/** Convert a hex color to an ANSI foreground escape. */
function hexToAnsiFg(hex) {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `\x1b[38;2;${r};${g};${b}m`;
}
/**
 * Component that renders a complete assistant message.
 *
 * Renders thinking blocks and text blocks as visually distinct cards. Model
 * identity is kept as a heading accent only; card surfaces come from the theme
 * so prompt/response/tool blocks stay visually consistent across models.
 */
export class AssistantMessageComponent extends Container {
    constructor(message, hideThinkingBlock = false, markdownTheme = getMarkdownTheme(), hiddenThinkingLabel = "Thinking...", modelAccent, modelGlyph) {
        super();
        this.hasToolCalls = false;
        this.hideThinkingBlock = hideThinkingBlock;
        this.markdownTheme = markdownTheme;
        this.hiddenThinkingLabel = hiddenThinkingLabel;
        this.modelAccent = modelAccent;
        this.modelGlyph = modelGlyph;
        this.contentContainer = new Container();
        this.addChild(this.contentContainer);
        if (message) {
            this.updateContent(message);
        }
    }
    invalidate() {
        super.invalidate();
        if (this.lastMessage) {
            this.updateContent(this.lastMessage);
        }
    }
    setHideThinkingBlock(hide) {
        this.hideThinkingBlock = hide;
        if (this.lastMessage) {
            this.updateContent(this.lastMessage);
        }
    }
    setHiddenThinkingLabel(label) {
        this.hiddenThinkingLabel = label;
        if (this.lastMessage) {
            this.updateContent(this.lastMessage);
        }
    }
    setModelAccent(hex) {
        this.modelAccent = hex;
        if (this.lastMessage) {
            this.updateContent(this.lastMessage);
        }
    }
    render(width) {
        const lines = super.render(width);
        if (this.hasToolCalls || lines.length === 0) {
            return lines;
        }
        lines[0] = OSC133_ZONE_START + lines[0];
        lines[lines.length - 1] = OSC133_ZONE_END + OSC133_ZONE_FINAL + lines[lines.length - 1];
        return lines;
    }
    updateContent(message) {
        this.lastMessage = message;
        this.contentContainer.clear();
        const hasVisibleContent = message.content.some((c) => (c.type === "text" && c.text.trim()) || (c.type === "thinking" && c.thinking.trim()));
        if (!hasVisibleContent) {
            // Still show errors/aborted status even without content
            const hasToolCalls = message.content.some((c) => c.type === "toolCall");
            this.hasToolCalls = hasToolCalls;
            if (!hasToolCalls && message.stopReason === "aborted") {
                const msg = message.errorMessage && message.errorMessage !== "Request was aborted"
                    ? message.errorMessage
                    : "Operation aborted";
                this.addStatusCard(theme.fg("error", msg));
            }
            else if (!hasToolCalls && message.stopReason === "error") {
                this.addStatusCard(theme.fg("error", `Error: ${message.errorMessage || "Unknown error"}`));
            }
            return;
        }
        // Model accent ANSI escapes (pre-computed for reuse).
        const accentFg = this.modelAccent && theme.modelAdaptive ? hexToAnsiFg(this.modelAccent) : undefined;
        // Helper: model accent foreground color for headings.
        const accent = (s) => (accentFg ? `${accentFg}${s}\x1b[39m` : theme.fg("accent", s));
        // Card presentation comes from the theme (theme.cards), not hardcoded, so
        // the look is portable theme data. Headings mirror the PROMPT card.
        const cards = theme.cards;
        const glyph = cards.headingGlyph === "model" ? this.modelGlyph : cards.headingGlyph || undefined;
        const showHeadings = cards.showHeadings && !!glyph;
        const responseBg = cards.shadedSurfaces ? theme.modelAdaptiveBgFn(this.modelAccent, "cardBg") : undefined;
        const thinkingBg = cards.shadedSurfaces
            ? (s) => theme.bg(cards.thinkingShaded ? "thinkingBg" : "cardBg", s)
            : undefined;
        const bodyIndent = showHeadings ? cards.bodyIndent : cards.headingIndent;
        let responseHeadingRendered = false;
        let thoughtHeadingRendered = false;
        const addHeading = (card, label) => {
            if (!showHeadings)
                return;
            card.addChild(new Text(accent(theme.bold(`${glyph} ${label}`)), cards.headingIndent, 0));
        };
        // Render content blocks in order.
        let blocksAdded = 0;
        for (let i = 0; i < message.content.length; i++) {
            const content = message.content[i];
            // Insert a spacer before every card except the first when gaplessCards is off.
            // Placing spacers before cards (not after) avoids a trailing gap that would
            // double up with chatContainer-level spacers between components.
            if (!theme.cards.gaplessCards && blocksAdded > 0) {
                this.contentContainer.addChild(new Spacer(1));
            }
            // ── Text (response) block ─────────────────────────────────
            if (content.type === "text" && content.text.trim()) {
                const textCard = new Box(cards.cardPadX, cards.cardPadY, responseBg);
                if (!responseHeadingRendered) {
                    addHeading(textCard, cards.responseLabel);
                    responseHeadingRendered = true;
                }
                textCard.addChild(new Markdown(content.text.trim(), bodyIndent, 0, this.markdownTheme));
                this.contentContainer.addChild(textCard);
                blocksAdded++;
            }
            // ── Thinking block ───────────────────────────────────────
            else if (content.type === "thinking" && content.thinking.trim()) {
                if (this.hideThinkingBlock) {
                    const label = theme.italic(theme.fg("thinkingText", this.hiddenThinkingLabel));
                    this.contentContainer.addChild(new Text(label, bodyIndent, 0));
                }
                else {
                    const thinkingCard = new Box(cards.cardPadX, cards.cardPadY, thinkingBg);
                    if (!thoughtHeadingRendered && cards.showThoughtHeading) {
                        addHeading(thinkingCard, cards.thoughtLabel);
                        thoughtHeadingRendered = true;
                    }
                    thinkingCard.addChild(new Markdown(content.thinking.trim(), bodyIndent, 0, this.markdownTheme, {
                        color: (t) => theme.fg("thinkingText", t),
                        italic: true,
                    }));
                    this.contentContainer.addChild(thinkingCard);
                    blocksAdded++;
                }
            }
        }
        // ── Stop-reason status ───────────────────────────────────────
        const hasToolCalls = message.content.some((c) => c.type === "toolCall");
        this.hasToolCalls = hasToolCalls;
        if (!hasToolCalls) {
            if (message.stopReason === "aborted") {
                const msg = message.errorMessage && message.errorMessage !== "Request was aborted"
                    ? message.errorMessage
                    : "Operation aborted";
                this.addStatusCard(theme.fg("error", msg));
            }
            else if (message.stopReason === "error") {
                const errorMsg = message.errorMessage || "Unknown error";
                this.addStatusCard(theme.fg("error", `Error: ${errorMsg}`));
            }
        }
    }
    addStatusCard(message) {
        const cards = theme.cards;
        const bgFn = cards.shadedSurfaces ? (s) => theme.bg("cardBg", s) : undefined;
        const statusCard = new Box(cards.cardPadX, cards.cardPadY, bgFn);
        statusCard.addChild(new Text(message, cards.headingIndent, 0));
        this.contentContainer.addChild(statusCard);
    }
}
//# sourceMappingURL=assistant-message.js.map