import type { ParsedModelOutput } from "./types.ts";
export { toolCallParserRegistry } from "./parsers/index.ts";
/**
 * Parse raw model output into a typed ParsedModelOutput.
 */
export declare function parseModelOutput(content: string, parserId: string, reasoningContent?: string): ParsedModelOutput;
//# sourceMappingURL=tool-calls.d.ts.map