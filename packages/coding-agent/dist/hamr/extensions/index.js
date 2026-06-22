import { createPersistentEditorExtension } from "../persistent-editor.js";
import { hamrCardsExtension } from "./cards.js";
import { hamrContextExtension } from "./context.js";
import { hamrMemoryExtension } from "./memory.js";
import { hamrProvidersExtension } from "./providers.js";
import { hamrReadLoopGuardExtension } from "./read-loop-guard.js";
import { createHamrSubagentsExtension } from "./subagents.js";
/**
 * The default set of hamr extensions, composed for the CLI. Each is an
 * independent pi extension factory — the monolith is gone, so a consumer can
 * include/exclude pieces (or eventually install them as separate packages) to
 * assemble their own agent. Subagents spawn child sessions with this same set
 * (resolved lazily to avoid an import cycle).
 */
export const hamrDefaultExtensions = [
    hamrProvidersExtension,
    hamrMemoryExtension,
    hamrCardsExtension,
    createHamrSubagentsExtension(() => hamrDefaultExtensions),
    createPersistentEditorExtension(),
    hamrReadLoopGuardExtension,
    hamrContextExtension,
];
export { hamrCardsExtension } from "./cards.js";
export { hamrMemoryExtension } from "./memory.js";
export { hamrProvidersExtension } from "./providers.js";
export { createHamrSubagentsExtension } from "./subagents.js";
//# sourceMappingURL=index.js.map