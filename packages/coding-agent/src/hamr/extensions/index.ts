import type { ExtensionFactory } from "../../core/extensions/types.ts";
import { createPersistentEditorExtension } from "../persistent-editor.ts";
import { hamrMemoryExtension } from "./memory.ts";
import { hamrProvidersExtension } from "./providers.ts";
import { createHamrSubagentsExtension } from "./subagents.ts";
import { hamrReadLoopGuardExtension } from "./read-loop-guard.ts";

/**
 * The default set of hamr extensions, composed for the CLI. Each is an
 * independent pi extension factory — the monolith is gone, so a consumer can
 * include/exclude pieces (or eventually install them as separate packages) to
 * assemble their own agent. Subagents spawn child sessions with this same set
 * (resolved lazily to avoid an import cycle).
 */
export const hamrDefaultExtensions: ExtensionFactory[] = [
	hamrProvidersExtension,
	hamrMemoryExtension,
	createHamrSubagentsExtension(() => hamrDefaultExtensions),
	createPersistentEditorExtension(),
	hamrReadLoopGuardExtension,
];

export { hamrMemoryExtension } from "./memory.ts";
export { hamrProvidersExtension } from "./providers.ts";
export { createHamrSubagentsExtension } from "./subagents.ts";
