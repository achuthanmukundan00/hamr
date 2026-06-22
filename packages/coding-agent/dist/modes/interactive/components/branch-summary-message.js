import { Box, Markdown, Text } from "@hamr/tui";
import { getMarkdownTheme, theme } from "../theme/theme.js";
import { keyText } from "./keybinding-hints.js";
/**
 * Component that renders a branch summary message with collapsed/expanded state.
 * Uses same background color as custom messages for visual consistency.
 */
export class BranchSummaryMessageComponent extends Box {
    constructor(message, markdownTheme = getMarkdownTheme()) {
        super(theme.cards.cardPadX, theme.cards.cardPadY, (t) => theme.bg("customMessageBg", t));
        this.expanded = false;
        this.message = message;
        this.markdownTheme = markdownTheme;
        this.updateDisplay();
    }
    setExpanded(expanded) {
        this.expanded = expanded;
        this.updateDisplay();
    }
    invalidate() {
        super.invalidate();
        this.updateDisplay();
    }
    updateDisplay() {
        this.clear();
        const cards = theme.cards;
        const label = theme.fg("customMessageLabel", `\x1b[1m[branch]\x1b[22m`);
        this.addChild(new Text(label, cards.headingIndent, 0));
        if (this.expanded) {
            const header = "**Branch Summary**\n\n";
            this.addChild(new Markdown(header + this.message.summary, cards.bodyIndent, 0, this.markdownTheme, {
                color: (text) => theme.fg("customMessageText", text),
            }));
        }
        else {
            this.addChild(new Text(theme.fg("customMessageText", "Branch summary (") +
                theme.fg("dim", keyText("app.tools.expand")) +
                theme.fg("customMessageText", " to expand)"), cards.bodyIndent, 0));
        }
    }
}
//# sourceMappingURL=branch-summary-message.js.map