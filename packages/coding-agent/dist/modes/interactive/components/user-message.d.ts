import { Container, type MarkdownTheme } from "@hamr/tui";
/**
 * Component that renders a user message with a branded PROMPT card header.
 *
 * Every user message was sent to a specific model — the card heading always
 * shows that model's glyph + "PROMPT" so you can see which model you prompted
 * even after mid-session model switches. The heading color reflects the model's
 * brand accent when modelAdaptive is on, or the theme accent when off.
 */
export declare class UserMessageComponent extends Container {
    private contentBox;
    constructor(text: string, markdownTheme?: MarkdownTheme, modelAccent?: string, modelGlyph?: string);
    render(width: number): string[];
}
//# sourceMappingURL=user-message.d.ts.map