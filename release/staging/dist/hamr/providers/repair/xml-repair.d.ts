/**
 * XML repair — bounded auto-recovery for Qwen-style XML tool calls.
 *
 * Qwen-family models emit `<tool_call>` blocks in XML format. Local
 * models frequently produce:
 * - Unclosed `<tool_call>` tags
 * - Leaked `<thinking>` / `<think>` tags inside tool calls
 * - Mixed XML + text content where thinking bleeds into tool blocks
 * - Nested `<tool_call>` with missing closing function tags
 *
 * Each repair is recorded in `fixes[]` for debugging. Returns `null` when
 * the input is unrepairable.
 */
export interface RepairResult {
    repaired: string;
    fixes: string[];
}
/**
 * Attempt to repair malformed XML tool-call text from a local model.
 *
 * Repairs applied:
 * 1. Strip leaked reasoning tags inside tool-call blocks
 * 2. Close unclosed `<tool_call>` tags
 * 3. Close unclosed `<function=...>` tags
 * 4. Close unclosed `<parameter=...>` tags
 * 5. Strip stray text between tool-call segments
 *
 * Returns `null` if the string is unrepairable (empty or garbage).
 */
export declare function repairXml(raw: string): RepairResult | null;
//# sourceMappingURL=xml-repair.d.ts.map