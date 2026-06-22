import { Container, type MarkdownTheme } from "@hamr/tui";
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
export declare class UserMessageComponent extends Container {
    constructor(text: string, markdownTheme?: MarkdownTheme, _modelAccent?: string, _modelGlyph?: string);
    render(width: number): string[];
}
//# sourceMappingURL=user-message.d.ts.map