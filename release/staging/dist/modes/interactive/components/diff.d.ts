import { type Component } from "@hamr/tui";
import { type ThemeBg } from "../theme/theme.ts";
/** Detect whether a blob of text is a raw git/unified diff. */
export declare function looksLikeUnifiedDiff(text: string): boolean;
export interface RenderDiffOptions {
    /** File path used to choose a syntax-highlighting language. */
    filePath?: string;
    /** Treat input as a raw git/unified diff rather than the internal format. */
    unified?: boolean;
    /**
     * Background the diff is painted onto. When the diff sits inside a shaded
     * surface (e.g. a tool card), pass that surface's background so the band
     * lines restore it after the colored band instead of resetting to the
     * terminal default — otherwise the surrounding padding shows a mismatched
     * strip on either side of the band.
     */
    surroundBg?: ThemeBg;
}
/**
 * Create a width-aware diff component. Use this everywhere a diff is shown
 * (file edits and git diffs) so the presentation stays consistent.
 */
export declare function createDiffComponent(diffText: string, options?: RenderDiffOptions): Component;
//# sourceMappingURL=diff.d.ts.map