/**
 * Subagents extension: the `delegate_subagents` tool for parallel/chain/stages
 * execution with bounded concurrency, live observability, and memory-safe state.
 *
 * Workers are spawned as isolated child `hamr` processes (`hamr --mode json -p`)
 * and the parent parses JSONL events for live updates. Full logs are persisted
 * to disk; only bounded recent events and output tails are kept in memory.
 *
 * Modes:
 *   - subtasks (serial, backward-compatible legacy)
 *   - tasks (parallel batch with bounded concurrency)
 *   - chain (serial with {previous} placeholder)
 *   - stages (serial stages, each parallel or chain internally)
 */
import type { ExtensionFactory } from "../../core/extensions/types.ts";
/** Marks the subagents factory so a parent can re-create it at depth + 1 for workers. */
export declare const HAMR_SUBAGENTS_FACTORY: unique symbol;
export declare function createHamrSubagentsExtension(_getChildExtensions: () => ExtensionFactory[], depth?: number): ExtensionFactory;
//# sourceMappingURL=subagents.d.ts.map