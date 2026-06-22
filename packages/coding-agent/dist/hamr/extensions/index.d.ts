import type { ExtensionFactory } from "../../core/extensions/types.ts";
/**
 * The default set of hamr extensions, composed for the CLI. Each is an
 * independent pi extension factory — the monolith is gone, so a consumer can
 * include/exclude pieces (or eventually install them as separate packages) to
 * assemble their own agent. Subagents spawn child sessions with this same set
 * (resolved lazily to avoid an import cycle).
 */
export declare const hamrDefaultExtensions: ExtensionFactory[];
export { hamrCardsExtension } from "./cards.ts";
export { hamrMemoryExtension } from "./memory.ts";
export { hamrProvidersExtension } from "./providers.ts";
export { createHamrSubagentsExtension } from "./subagents.ts";
//# sourceMappingURL=index.d.ts.map