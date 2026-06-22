import type { AssistantMessage } from "@hamr/ai";
export declare function modelKey(provider: string, model: string): string;
export declare function contentText(content: unknown): string;
export declare function getAssistantText(message: AssistantMessage): string;
export declare function getThinkingText(message: AssistantMessage): string | undefined;
export declare function hasToolCalls(message: AssistantMessage): boolean;
export declare function fileHints(text: string): string[];
//# sourceMappingURL=helpers.d.ts.map