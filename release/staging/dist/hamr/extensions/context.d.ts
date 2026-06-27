import type { ExtensionFactory } from "../../core/extensions/types.ts";
import type { ThemeColor } from "../../modes/interactive/theme/theme.ts";
interface Category {
    name: string;
    tokens: number;
    color: ThemeColor;
}
export interface ContextBreakdown {
    modelName: string | undefined;
    modelId: string | undefined;
    contextWindow: number;
    /** Total estimated tokens (from last API usage + trailing estimate, or pure estimate). */
    tokens: number | null;
    percent: number | null;
    /** Per-category breakdown in display order. */
    categories: Category[];
    /** True when the total is based on an actual API response. */
    fromApi: boolean;
}
export declare const hamrContextExtension: ExtensionFactory;
export {};
//# sourceMappingURL=context.d.ts.map