import { Container, Markdown } from "@hamr/tui";
import { getMarkdownTheme } from "../theme/theme.js";
const OSC133_ZONE_START = "\x1b]133;A\x07";
const OSC133_ZONE_END = "\x1b]133;B\x07";
const OSC133_ZONE_FINAL = "\x1b]133;C\x07";
/**
 * Plain/fallback component for user messages.
 *
 * Renders the message text as Markdown with no card structure, no heading,
 * and no model accent. Extensions (such as hamr-cards) can register a role
 * renderer to wrap this in a themed card if desired.
 *
 * The constructor keeps modelAccent/modelGlyph params for API compatibility
 * but they are ignored in the plain fallback.
 */
export class UserMessageComponent extends Container {
    constructor(text, markdownTheme = getMarkdownTheme(), _modelAccent, _modelGlyph) {
        super();
        this.addChild(new Markdown(text, 1, 0, markdownTheme, undefined, { preserveOrderedListMarkers: true }));
    }
    render(width) {
        const lines = super.render(width);
        if (lines.length === 0) {
            return lines;
        }
        lines[0] = OSC133_ZONE_START + lines[0];
        lines[lines.length - 1] = OSC133_ZONE_END + OSC133_ZONE_FINAL + lines[lines.length - 1];
        return lines;
    }
}
//# sourceMappingURL=user-message.js.map