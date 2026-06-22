import type { AssistantMessage } from "@hamr/ai";
import { Container, type MarkdownTheme } from "@hamr/tui";
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
export declare class AssistantMessageComponent extends Container {
    private contentContainer;
    private hideThinkingBlock;
    private markdownTheme;
    private hiddenThinkingLabel;
    private lastMessage?;
    private hasToolCalls;
    private _modelAccent?;
    private _modelGlyph?;
    constructor(message?: AssistantMessage, hideThinkingBlock?: boolean, markdownTheme?: MarkdownTheme, hiddenThinkingLabel?: string, modelAccent?: string, modelGlyph?: string);
    invalidate(): void;
    setHideThinkingBlock(hide: boolean): void;
    setHiddenThinkingLabel(label: string): void;
    setModelAccent(_hex: string | undefined): void;
    render(width: number): string[];
    updateContent(message: AssistantMessage): void;
}
//# sourceMappingURL=assistant-message.d.ts.map