import type { AssistantMessage, AssistantMessageEvent } from "@hamr/ai";
import type { ExtensionContext, ExtensionFactory } from "../core/extensions/types.ts";
import type { HamrStartupConfig } from "./startup-config.ts";
export declare const parserByModel: Map<string, string>;
export declare function repairLocalToolCalls(message: AssistantMessage, ctx: ExtensionContext): AssistantMessage | undefined;
export declare function hasSubstantialContent(event: AssistantMessageEvent): boolean;
export declare function registerHamrProviders(pi: Parameters<ExtensionFactory>[0], config: HamrStartupConfig): Promise<void>;
//# sourceMappingURL=repair.d.ts.map