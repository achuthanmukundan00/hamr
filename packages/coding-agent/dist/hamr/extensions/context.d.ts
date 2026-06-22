import type { ExtensionFactory } from "../../core/extensions/types.ts";
export interface ContextBreakdown {
    modelName: string | undefined;
    modelId: string | undefined;
    contextWindow: number;
    tokens: number | null;
    percent: number | null;
    systemPrompt: number;
    skills: number;
    contextFiles: number;
    messagesAndTools: number | null;
}
export declare const hamrContextExtension: ExtensionFactory;
//# sourceMappingURL=context.d.ts.map