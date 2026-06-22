import { type Component } from "@hamr/tui";
import type { AgentSession } from "../../../core/agent-session.ts";
import type { ReadonlyFooterDataProvider } from "../../../core/footer-data-provider.ts";
export declare function formatContextPart(tokens: number | null | undefined, contextWindow: number, percent: number | null | undefined, compact: boolean): string | undefined;
/**
 * Format the accumulated cost for the status bar. Uses 3-decimal precision so
 * sub-cent spend (e.g. $0.003) is visible rather than rounding to $0.00.
 * Shows accumulated cost when present. Omits the segment only when there is
 * no prior cloud spend AND the current model is zero-priced (relay/local).
 */
export declare function formatCostPart(totalCost: number, inputPricePerMillion: number, usingSubscription: boolean): string | undefined;
export declare function formatCwdForFooter(cwd: string, home: string | undefined): string;
/**
 * Footer component that shows Hamr's synax-style single-line status:
 * activity on the left, context/spend/tokens/provider/model/thinking on the right.
 */
export declare class FooterComponent implements Component {
    private static readonly RAINBOW_LUT;
    private session;
    private footerData;
    private requestRender?;
    private animationTimer;
    constructor(session: AgentSession, footerData: ReadonlyFooterDataProvider, requestRender?: () => void);
    setSession(session: AgentSession): void;
    setAutoCompactEnabled(_enabled: boolean): void;
    invalidate(): void;
    dispose(): void;
    render(width: number): string[];
    private isAnimating;
    private startAnimationTimer;
    private stopAnimationTimer;
    private renderActivityText;
    private isMaxThinking;
    private renderRightSide;
    private getSessionUsage;
}
//# sourceMappingURL=footer.d.ts.map