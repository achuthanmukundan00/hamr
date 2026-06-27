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
import type { Usage } from "@hamr/ai";
import type { ExtensionFactory } from "../../core/extensions/types.ts";
/** Marks the subagents factory so a parent can re-create it at depth + 1 for workers. */
export declare const HAMR_SUBAGENTS_FACTORY: unique symbol;
interface ActivityEvent {
    ts: number;
    type: string;
    data: string;
}
interface WorkerState {
    workerId: string;
    taskPreview: string;
    cwd: string;
    status: "queued" | "running" | "done" | "failed" | "aborted";
    pid?: number;
    model?: string;
    startedAt?: number;
    endedAt?: number;
    usage: Usage;
    estimatedUsage?: boolean;
    stopReason?: string;
    errorMessage?: string;
    lastActivity?: string;
    lastTool?: string;
    /** Fresh activity summary extracted from worker output or tool activity. */
    lastSummary?: string;
    /** Summary currently shown in the TUI. It rotates slowly to avoid log-like churn. */
    displaySummary?: string;
    displaySummaryUpdatedAt?: number;
    /** Last N tool calls for loop detection. */
    toolCallHistory: string[];
    /** Set when the worker appears to be in a repeated-tool-call loop. */
    looping?: boolean;
    recentEvents: ActivityEvent[];
    pendingFlush: string[];
    flushTimer?: ReturnType<typeof setInterval>;
    outputTail: string;
    finalOutput?: string;
    logPath: string;
    resultPath?: string;
    /** Live workers from a nested delegate_subagents call (recursive). */
    nestedWorkers?: LiveWorkerView[];
    /** Header text for nested subagent swarm (e.g. "parallel 2/5 · 3 active"). */
    nestedHeader?: string;
}
interface LiveWorkerView {
    workerId: string;
    task: string;
    status: WorkerState["status"] | "timeout";
    model?: string;
    displaySummary?: string;
    lastSummary?: string;
    lastTool?: string;
    looping?: boolean;
    usage: Usage;
    /** Worker has output validation warnings (yellow dot on final). */
    hasWarnings?: boolean;
    /** Worker has high-severity output warnings (red dot on final). */
    hasHighWarnings?: boolean;
    /** Nested sub-worker views when this worker spawns its own subagents. */
    nestedWorkers?: LiveWorkerView[];
    /** Header text for nested swarm (e.g. "parallel 2/5 · 3 active · ↓45K tok"). */
    nestedHeader?: string;
}
interface ValidationWarning {
    type: "missing_file" | "empty_output" | "truncated_output" | "self_contradiction" | "suspicious_pattern";
    message: string;
    severity: "low" | "medium" | "high";
}
interface ValidationResult {
    passed: boolean;
    warnings: ValidationWarning[];
    /** 0.0–1.0 heuristic confidence score */
    confidence: number;
}
type WorkerOutcome = {
    status: "done";
    workerId: string;
    task: string;
    text: string;
    usage: Usage;
    model?: string;
    estimatedUsage?: boolean;
    stopReason?: string;
    validation?: ValidationResult;
} | {
    status: "failed";
    workerId: string;
    task: string;
    error: string;
    text: string;
    validation?: ValidationResult;
} | {
    status: "aborted";
    workerId: string;
    task: string;
    reason: "user" | "parent" | "timeout";
} | {
    status: "timeout";
    workerId: string;
    task: string;
    partialText: string;
    validation?: ValidationResult;
};
/**
 * Validate subagent output before it is merged into the parent session.
 *
 * Checks:
 * 1. Non-empty, non-truncated output
 * 2. File references against the actual file-system under the worker's cwd
 * 3. Self-contradiction heuristics
 *
 * Returns a confidence score (0.0–1.0) and a list of warnings.
 */
declare function validateWorkerOutput(outcome: WorkerOutcome, cwd: string): ValidationResult;
declare function createWorkerState(workerId: string, task: string, cwd: string, logPath: string): WorkerState;
declare function pushEvent(ws: WorkerState, event: Record<string, unknown>): void;
export declare function createHamrSubagentsExtension(_getChildExtensions: () => ExtensionFactory[], depth?: number): ExtensionFactory;
interface WorkerProcessSummary {
    exitCode: number;
    exitSignal?: NodeJS.Signals | null;
    wasAborted: boolean;
    stderr: string;
    outputText: string;
    usage: Usage;
    model?: string;
    estimatedUsage: boolean;
    stopReason?: string;
    errorMessage?: string;
    stdoutParseErrors: number;
    invalidStdout: string;
    spawnError?: string;
}
interface WorkerProcessEventState {
    outputText: string;
    usage: Usage;
    model?: string;
    estimatedUsage: boolean;
    stopReason?: string;
    errorMessage?: string;
    assistantMessageEndCount: number;
    stdoutParseErrors: number;
    invalidStdout: string;
}
declare function createWorkerProcessEventState(): WorkerProcessEventState;
declare function recordWorkerProcessEvent(state: WorkerProcessEventState, event: Record<string, unknown>): void;
declare function buildWorkerOutcomeFromChildSummary(workerId: string, task: string, summary: WorkerProcessSummary): WorkerOutcome;
export declare const _testExports: {
    pushEvent: typeof pushEvent;
    validateWorkerOutput: typeof validateWorkerOutput;
    createWorkerState: typeof createWorkerState;
    createWorkerProcessEventState: typeof createWorkerProcessEventState;
    recordWorkerProcessEvent: typeof recordWorkerProcessEvent;
    buildWorkerOutcomeFromChildSummary: typeof buildWorkerOutcomeFromChildSummary;
};
export {};
//# sourceMappingURL=subagents.d.ts.map