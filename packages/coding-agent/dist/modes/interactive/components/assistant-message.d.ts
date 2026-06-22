import type { AssistantMessage } from "@hamr/ai";
import { Container, type MarkdownTheme } from "@hamr/tui";
/**
 * Component that renders a complete assistant message.
 *
 * Renders thinking blocks and text blocks as visually distinct cards. Model
 * identity is kept as a heading accent only; card surfaces come from the theme
 * so prompt/response/tool blocks stay visually consistent across models.
 */
export declare class AssistantMessageComponent extends Container {
    private contentContainer;
    private hideThinkingBlock;
    private markdownTheme;
    private hiddenThinkingLabel;
    private lastMessage?;
    private hasToolCalls;
    private modelAccent?;
    private modelGlyph?;
    constructor(message?: AssistantMessage, hideThinkingBlock?: boolean, markdownTheme?: MarkdownTheme, hiddenThinkingLabel?: string, modelAccent?: string, modelGlyph?: string);
    invalidate(): void;
    setHideThinkingBlock(hide: boolean): void;
    setHiddenThinkingLabel(label: string): void;
    setModelAccent(hex: string | undefined): void;
    render(width: number): string[];
    updateContent(message: AssistantMessage): void;
    private addStatusCard;
}
//# sourceMappingURL=assistant-message.d.ts.map