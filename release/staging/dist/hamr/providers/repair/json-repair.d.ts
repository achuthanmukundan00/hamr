/**
 * JSON repair — bounded auto-recovery for local-model tool-call JSON.
 *
 * Local models frequently emit malformed JSON with:
 * - Trailing commas: {"name": "read", "args": {"path": "x",}}
 * - Unescaped inner quotes: {"query": "find "foo" in bar"}
 * - Truncated objects: {"name": "bash", "args": {"command": "npm te
 * - Missing closing braces: {"name": "edit", "args": {"path": "x", "oldStr": "y"
 * - Mixed format: prose + partial JSON blocks
 *
 * Each repair is recorded in `fixes[]` for debugging. Returns `null` when
 * the input is unrepairable (garbage, empty, or too broken to guess).
 */
export interface RepairResult {
    repaired: string;
    fixes: string[];
}
/**
 * Attempt to repair malformed JSON produced by a local model.
 *
 * Repairs are applied in increasing order of invasiveness:
 * 1. Trim whitespace and surrounding noise
 * 2. Fix trailing commas
 * 3. Balance braces/brackets
 * 4. Heuristic inner-quote repair
 *
 * Returns `null` if the string is unrepairable.
 */
export declare function repairJson(raw: string): RepairResult | null;
//# sourceMappingURL=json-repair.d.ts.map