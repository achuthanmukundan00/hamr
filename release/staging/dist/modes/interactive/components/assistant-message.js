import { Container, Markdown, Text } from "@hamr/tui";
import { getMarkdownTheme } from "../theme/theme.js";
const OSC133_ZONE_START = "\x1b]133;A\x07";
const OSC133_ZONE_END = "\x1b]133;B\x07";
const OSC133_ZONE_FINAL = "\x1b]133;C\x07";
/**
 * Plain/fallback component for assistant messages.
 *
 * Renders text blocks as plain Markdown and thinking blocks as plain text.
 * No card structure, headings, or model accent — intended as the minimal
 * fallback when no extension registers a role renderer.
 *
 * Extensions (such as hamr-cards) can register a role renderer to wrap
 * blocks in themed cards with headings and model branding.
 */
export class AssistantMessageComponent extends Container {
    constructor(message, hideThinkingBlock = false, markdownTheme = getMarkdownTheme(), hiddenThinkingLabel = "Thinking...", modelAccent, modelGlyph) {
        super();
        this.hasToolCalls = false;
        this.hideThinkingBlock = hideThinkingBlock;
        this.markdownTheme = markdownTheme;
        this.hiddenThinkingLabel = hiddenThinkingLabel;
        this._modelAccent = modelAccent;
        this._modelGlyph = modelGlyph;
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
    setModelAccent(_hex) {
        this._modelAccent = _hex;
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
            const hasToolCalls = message.content.some((c) => c.type === "toolCall");
            this.hasToolCalls = hasToolCalls;
            if (!hasToolCalls && message.stopReason === "aborted") {
                const msg = message.errorMessage && message.errorMessage !== "Request was aborted"
                    ? message.errorMessage
                    : "Operation aborted";
                this.contentContainer.addChild(new Text(msg, 1, 0));
            }
            else if (!hasToolCalls && message.stopReason === "error") {
                this.contentContainer.addChild(new Text(`Error: ${message.errorMessage || "Unknown error"}`, 1, 0));
            }
            return;
        }
        // Render content blocks in order — plain, no cards.
        for (const content of message.content) {
            if (content.type === "text" && content.text.trim()) {
                this.contentContainer.addChild(new Markdown(content.text.trim(), 1, 0, this.markdownTheme));
            }
            else if (content.type === "thinking" && content.thinking.trim()) {
                if (this.hideThinkingBlock) {
                    this.contentContainer.addChild(new Text(this.hiddenThinkingLabel, 1, 0));
                }
                else {
                    this.contentContainer.addChild(new Text(content.thinking.trim(), 1, 0));
                }
            }
        }
        const hasToolCalls = message.content.some((c) => c.type === "toolCall");
        this.hasToolCalls = hasToolCalls;
        if (!hasToolCalls) {
            if (message.stopReason === "aborted") {
                const msg = message.errorMessage && message.errorMessage !== "Request was aborted"
                    ? message.errorMessage
                    : "Operation aborted";
                this.contentContainer.addChild(new Text(msg, 1, 0));
            }
            else if (message.stopReason === "error") {
                const errorMsg = message.errorMessage || "Unknown error";
                this.contentContainer.addChild(new Text(`Error: ${errorMsg}`, 1, 0));
            }
        }
    }
}
//# sourceMappingURL=assistant-message.js.map