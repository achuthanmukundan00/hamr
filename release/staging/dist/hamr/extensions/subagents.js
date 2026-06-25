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
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { Container, Markdown, Spacer, Text, truncateToWidth } from "@hamr/tui";
import { Type } from "typebox";
import { defineTool } from "../../core/extensions/types.js";
import { getDefaultSessionDirPath } from "../../core/session-manager.js";
import { getMarkdownTheme } from "../../modes/interactive/theme/theme.js";
import { killProcessTree, killTrackedDetachedChildren, trackDetachedChildPid, untrackDetachedChildPid, } from "../../utils/shell.js";
import { isCloudProvider, loadHamrStartupConfig } from "../startup-config.js";
// ─── Environment configuration ───────────────────────────────────────────────
const ENV_MAX_TASKS = Number.parseInt(process.env.HAMR_SUBAGENT_MAX_TASKS ?? "64", 10) || 64;
const ENV_HARD_MAX_TASKS = Number.parseInt(process.env.HAMR_SUBAGENT_HARD_MAX_TASKS ?? "256", 10) || 256;
const ENV_MAX_CONCURRENCY = Number.parseInt(process.env.HAMR_SUBAGENT_MAX_CONCURRENCY ?? "64", 10) || 64;
/** Global budget cap across the entire recursive subagent tree. */
const ENV_TOTAL_BUDGET = Number.parseInt(process.env.HAMR_SUBAGENT_BUDGET ?? "1024", 10) || 1024;
/** Env var passed to child processes with the remaining budget for their subtree. */
const ENV_TREE_REMAINING = "HAMR_SUBAGENT_TREE_REMAINING";
/** Env var passed to child processes pointing to the serialized parent config. */
const ENV_CHILD_CONFIG = "HAMR_CHILD_CONFIG";
/** Per-worker step timeout in ms (default: 5 min). */
const ENV_STEP_TIMEOUT_MS = Number.parseInt(process.env.HAMR_SUBAGENT_STEP_TIMEOUT_MS ?? "300000", 10) || 300000;
/** Per-run total timeout in ms (default: 30 min). */
const ENV_TOTAL_TIMEOUT_MS = Number.parseInt(process.env.HAMR_SUBAGENT_TOTAL_TIMEOUT_MS ?? "1800000", 10) || 1800000;
/**
 * In-memory budget tracking.
 * Root process reads from HAMR_SUBAGENT_BUDGET env; children read from
 * HAMR_SUBAGENT_TREE_REMAINING env (set by their parent).
 */
let treeBudgetRemaining = Number.parseInt(process.env[ENV_TREE_REMAINING] ?? "", 10) || ENV_TOTAL_BUDGET;
const OUTPUT_TAIL_BYTES = Number.parseInt(process.env.HAMR_SUBAGENT_OUTPUT_TAIL_BYTES ?? "32768", 10) || 32768;
const EVENTS_IN_MEMORY = Number.parseInt(process.env.HAMR_SUBAGENT_EVENTS_IN_MEMORY ?? "40", 10) || 40;
/** Flush events to disk every N events or every this many ms, whichever comes first. */
const FLUSH_BATCH_SIZE = 10;
const FLUSH_INTERVAL_MS = 500;
const LOG_DIR_BASE = process.env.HAMR_SUBAGENT_LOG_DIR ?? ".hamr/subagents";
/** Max completed runs to retain in memory for the status widget. */
const MAX_ACTIVE_RUNS = 50;
/** Marks the subagents factory so a parent can re-create it at depth + 1 for workers. */
export const HAMR_SUBAGENTS_FACTORY = Symbol.for("hamr.subagents.factory");
/** Recursion bound. Root = 0; at this depth the worker gets no delegate tool. */
const MAX_DEPTH = 3;
// ─── Orphaned child-config cleanup ───────────────────────────────────────────
// Child config temp files carry the provider API key and CF-Access credentials.
// Track them so a parent crash (SIGKILL/OOM/segfault) can't leave secrets in /tmp.
// `killTrackedDetachedChildren` (registered as a process-exit hook below) also
// unlinks any still-registered config paths.
const orphanedConfigPaths = new Set();
let parentExitHookInstalled = false;
function registerOrphanedConfigForCleanup(configPath) {
    orphanedConfigPaths.add(configPath);
    if (!parentExitHookInstalled) {
        parentExitHookInstalled = true;
        const cleanup = () => {
            for (const p of orphanedConfigPaths) {
                try {
                    fs.unlinkSync(p);
                }
                catch {
                    /* best-effort */
                }
            }
            orphanedConfigPaths.clear();
            killTrackedDetachedChildren();
        };
        process.once("exit", cleanup);
        process.once("SIGINT", () => {
            cleanup();
            process.exit(130);
        });
        process.once("SIGTERM", () => {
            cleanup();
            process.exit(143);
        });
    }
}
function unregisterOrphanedConfigForCleanup(configPath) {
    orphanedConfigPaths.delete(configPath);
}
const EMPTY_USAGE = {
    input: 0,
    output: 0,
    cacheRead: 0,
    cacheWrite: 0,
    totalTokens: 0,
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
};
// ─── Global state ────────────────────────────────────────────────────────────
const activeRuns = new Map();
let runCounter = 0;
// ─── Helpers ─────────────────────────────────────────────────────────────────
function nextRunId() {
    return `run-${Date.now().toString(36)}-${(++runCounter).toString(36)}`;
}
function formatTokens(tokens) {
    if (tokens >= 1_000_000)
        return `${(tokens / 1_000_000).toFixed(1)}M`;
    if (tokens >= 10_000)
        return `${Math.round(tokens / 1000)}K`;
    if (tokens >= 1000)
        return `${(tokens / 1000).toFixed(1)}K`;
    return `${tokens}`;
}
/** Deep-clone a value via JSON round-trip. Returns the original on failure (BigInt, circular refs). */
function safeJsonClone(value) {
    try {
        return JSON.parse(JSON.stringify(value));
    }
    catch {
        return value;
    }
}
function clamp(value, min, max) {
    return Math.max(min, Math.min(max, value));
}
function padWorkerId(idx, total) {
    const width = String(total).length;
    return String(idx).padStart(width, "0");
}
// ─── Tool call formatting (mirrors built-in tool renderers) ──────────────────
function shortenHome(p) {
    const home = os.homedir();
    return p.startsWith(home) ? `~${p.slice(home.length)}` : p;
}
function formatToolCall(toolName, args, fg) {
    switch (toolName) {
        case "bash": {
            const cmd = args.command || "...";
            const preview = cmd.length > 60 ? `${cmd.slice(0, 60)}…` : cmd;
            return fg("muted", "$ ") + fg("toolOutput", preview);
        }
        case "read": {
            const rawPath = (args.file_path || args.path || "...");
            const filePath = shortenHome(rawPath);
            const offset = args.offset;
            const limit = args.limit;
            let text = fg("accent", filePath);
            if (offset !== undefined || limit !== undefined) {
                const start = offset ?? 1;
                const end = limit !== undefined ? start + limit - 1 : "";
                text += fg("warning", `:${start}${end ? `-${end}` : ""}`);
            }
            return fg("muted", "read ") + text;
        }
        case "write": {
            const rawPath = (args.file_path || args.path || "...");
            const lines = (args.content || "").split("\n").length;
            let text = fg("muted", "write ") + fg("accent", shortenHome(rawPath));
            if (lines > 1)
                text += fg("dim", ` (${lines} lines)`);
            return text;
        }
        case "edit": {
            const rawPath = (args.file_path || args.path || "...");
            return fg("muted", "edit ") + fg("accent", shortenHome(rawPath));
        }
        case "ls": {
            const rawPath = (args.path || ".");
            return fg("muted", "ls ") + fg("accent", shortenHome(rawPath));
        }
        case "find": {
            const pattern = (args.pattern || "*");
            const rawPath = (args.path || ".");
            return fg("muted", "find ") + fg("accent", pattern) + fg("dim", ` in ${shortenHome(rawPath)}`);
        }
        case "grep": {
            const pattern = (args.pattern || "");
            const rawPath = (args.path || ".");
            return fg("muted", "grep ") + fg("accent", `/${pattern}/`) + fg("dim", ` in ${shortenHome(rawPath)}`);
        }
        default: {
            const argsStr = JSON.stringify(args);
            const preview = argsStr.length > 50 ? `${argsStr.slice(0, 50)}…` : argsStr;
            return fg("accent", toolName) + fg("dim", ` ${preview}`);
        }
    }
}
function aggregateAllRuns() {
    const stats = {
        total: 0,
        queued: 0,
        running: 0,
        done: 0,
        failed: 0,
        aborted: 0,
        totalTokens: 0,
        totalCost: 0,
    };
    for (const run of activeRuns.values()) {
        stats.total += run.total;
        stats.queued += run.queued;
        stats.running += run.running;
        stats.done += run.done;
        stats.failed += run.failed;
        stats.aborted += run.aborted;
        stats.totalTokens += run.usage.totalTokens ?? 0;
        stats.totalCost += run.usage.cost?.total ?? 0;
    }
    return stats;
}
const RUNNING_FRAMES = ["◐", "◓", "◑", "◒"];
function animationFrame() {
    return RUNNING_FRAMES[Math.floor(Date.now() / 180) % RUNNING_FRAMES.length];
}
function renderStatusLine() {
    const stats = aggregateAllRuns();
    if (stats.total === 0)
        return undefined;
    const parts = [];
    if (stats.running > 0)
        parts.push(`${animationFrame()} ${stats.running} running`);
    if (stats.queued > 0)
        parts.push(`${stats.queued} queued`);
    if (stats.done > 0)
        parts.push(`✓ ${stats.done} done`);
    if (stats.failed > 0)
        parts.push(`✕ ${stats.failed} failed`);
    if (stats.aborted > 0)
        parts.push(`⊘ ${stats.aborted} aborted`);
    if (stats.totalTokens > 0)
        parts.push(`↓${formatTokens(stats.totalTokens)} tok`);
    if (stats.totalCost > 0)
        parts.push(`$${stats.totalCost.toFixed(4)}`);
    return `subagents ${parts.join(" · ")}`;
}
function evictOldRuns() {
    if (activeRuns.size <= MAX_ACTIVE_RUNS)
        return;
    const runs = [...activeRuns.entries()]
        .filter(([, r]) => r.endedAt != null)
        .sort((a, b) => (a[1].endedAt ?? Infinity) - (b[1].endedAt ?? Infinity));
    for (let i = 0; i < runs.length - MAX_ACTIVE_RUNS; i++) {
        activeRuns.delete(runs[i][0]);
    }
}
class AgentStatusWidget {
    constructor(tui, theme) {
        this.lastLine = "";
        this.tui = tui;
        this.theme = theme;
        this.interval = setInterval(() => {
            const line = renderStatusLine() ?? "";
            if (line !== this.lastLine) {
                this.lastLine = line;
                this.tui.requestRender();
            }
        }, 180);
    }
    render(width) {
        const line = renderStatusLine() ?? this.lastLine;
        if (!line)
            return [];
        const colored = ` ${this.theme.fg("muted", line)}`;
        return [truncateToWidth(colored, width)];
    }
    invalidate() { }
    dispose() {
        if (this.interval) {
            clearInterval(this.interval);
            this.interval = undefined;
        }
    }
}
function updateStatusWidget(ctx) {
    if (ctx.mode !== "tui")
        return;
    const widget = renderStatusLine() ? (tui, theme) => new AgentStatusWidget(tui, theme) : undefined;
    ctx.ui.setWidget("hamr.subagents.status", widget, { placement: "aboveEditor" });
}
// ─── Disk persistence ────────────────────────────────────────────────────────
function ensureLogDir(runId, cwd) {
    const base = path.resolve(cwd, LOG_DIR_BASE, "runs", runId);
    fs.mkdirSync(path.join(base, "workers"), { recursive: true, mode: 0o700 });
    try {
        fs.chmodSync(base, 0o700);
    }
    catch {
        /* best-effort */
    }
    return base;
}
function appendNDJSON(filePath, lines) {
    try {
        // Event logs can carry tool I/O and file contents — keep them owner-only
        // (matches the 0o600 result files), not the default world-readable mode.
        fs.appendFileSync(filePath, lines.join(""), { encoding: "utf-8", mode: 0o600 });
    }
    catch {
        // best-effort
    }
}
/** Flush pending events that haven't been written to disk yet. */
function flushPendingEvents(ws) {
    if (ws.pendingFlush.length === 0)
        return;
    appendNDJSON(ws.logPath, ws.pendingFlush);
    ws.pendingFlush = [];
}
/** Flush any remaining pending events at worker completion / abort / crash. */
function flushWorkerLog(ws) {
    flushPendingEvents(ws);
}
/** Reload completed runs from disk on session resume so the status bar repopulates. */
function restoreRunsFromDisk(cwd, sessionId) {
    const base = path.resolve(cwd, LOG_DIR_BASE, "runs");
    if (!fs.existsSync(base))
        return;
    try {
        for (const runDir of fs.readdirSync(base, { withFileTypes: true })) {
            if (!runDir.isDirectory())
                continue;
            const runJsonPath = path.join(base, runDir.name, "run.json");
            if (!fs.existsSync(runJsonPath))
                continue;
            try {
                const data = JSON.parse(fs.readFileSync(runJsonPath, "utf-8"));
                if (!data.endedAt || activeRuns.has(data.runId))
                    continue;
                if (data.parentSessionId !== sessionId)
                    continue;
                const run = {
                    runId: data.runId,
                    mode: data.mode ?? "single",
                    total: data.total ?? 0,
                    queued: 0,
                    running: 0,
                    done: data.done ?? 0,
                    failed: data.failed ?? 0,
                    aborted: data.aborted ?? 0,
                    startedAt: new Date(data.startedAt).getTime(),
                    endedAt: new Date(data.endedAt).getTime(),
                    usage: data.usage ?? { ...EMPTY_USAGE },
                    logDir: path.dirname(runJsonPath),
                    workers: new Map(),
                    _cnt: {
                        queued: 0,
                        running: 0,
                        done: data.done ?? 0,
                        failed: data.failed ?? 0,
                        aborted: data.aborted ?? 0,
                        tok: data.usage?.totalTokens ?? 0,
                    },
                };
                activeRuns.set(data.runId, run);
            }
            catch {
                // corrupted run.json — skip
            }
        }
    }
    catch {
        // best-effort
    }
}
// ─── Bash-only fast path helper ──────────────────────────────────────────────
/** Returns true if the tools list qualifies for the bash-only fast path. */
function isBashFastPathTools(tools) {
    return (tools !== undefined &&
        tools.length > 0 &&
        tools.length <= 2 &&
        tools.every((t) => t === "bash" || t === "read") &&
        tools.includes("bash"));
}
/** Heuristic: does a string look like a file-system path (not a URL, version, etc.)? */
function looksLikeFilePath(s) {
    if (/^https?:\/\//i.test(s))
        return false;
    if (/^@?[a-z0-9-]+\/[@a-z0-9-]+@[\d.]+/.test(s))
        return false;
    if (/^v?\d+\.\d+\.\d+/.test(s))
        return false;
    if (!/\.[a-zA-Z0-9]{1,10}$/.test(s))
        return false;
    if (/^\d+\.\d+\.\d+$/.test(s))
        return false;
    return true;
}
/** Extract plausible file-system paths from arbitrary assistant text. */
function extractFileReferences(text) {
    const refs = new Set();
    // Backtick-wrapped candidates: `src/foo.ts`
    const backtickRe = /`([^`\n]{1,200})`/g;
    let match;
    while ((match = backtickRe.exec(text)) !== null) {
        const candidate = match[1].trim();
        if (looksLikeFilePath(candidate))
            refs.add(candidate);
    }
    // Path-like tokens in running text:  ./a/b.ts  ../c/d.ts  /abs/e/f.ts  a/b/c.ts
    const pathRe = /(?:^|\s|[`"'(\s])((?:\.{0,2}\/)?[\w.@-]+(?:\/[\w.@-]+)*\.[a-zA-Z0-9]{1,10})/g;
    while ((match = pathRe.exec(text)) !== null) {
        const candidate = match[1].trim();
        if (looksLikeFilePath(candidate))
            refs.add(candidate);
    }
    return [...refs];
}
/** Check output for self-contradictory patterns (heuristic, low-confidence). */
function checkSelfContradiction(text) {
    const warnings = [];
    const lower = text.toLowerCase();
    // Output mentions both no-errors and failures nearby
    if (/\b(?:no\s+)?error(?:s)?\b/.test(lower) && /\bfail(?:ed|ure)?\b/.test(lower)) {
        warnings.push({
            type: "self_contradiction",
            message: "Output mentions both error(s) and failure(s) – review for consistency.",
            severity: "low",
        });
    }
    // Output claims creation but also mentions missing items
    if (/\b(?:created?|wrote?|generated?|built?)\b/.test(lower) &&
        /\b(?:does\s+not\s+exist|not\s+found|cannot\s+find|no\s+such)\b/.test(lower)) {
        warnings.push({
            type: "self_contradiction",
            message: "Output claims creation but also mentions missing/non-existent items.",
            severity: "medium",
        });
    }
    return warnings;
}
function fileExistsRelative(cwd, fileRef) {
    try {
        const resolved = path.resolve(cwd, fileRef);
        return fs.existsSync(resolved);
    }
    catch {
        return false;
    }
}
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
function validateWorkerOutput(outcome, cwd) {
    const warnings = [];
    // Determine output text based on outcome status
    let text = "";
    if (outcome.status === "done" || outcome.status === "failed") {
        text = outcome.text;
    }
    else if (outcome.status === "timeout") {
        text = outcome.partialText;
    }
    // 1. Empty / missing output
    if (!text || text.trim().length === 0) {
        warnings.push({
            type: "empty_output",
            message: "Worker produced no output text.",
            severity: "high",
        });
        return { passed: false, warnings, confidence: 0.0 };
    }
    // 2. Truncated output
    if (text.length >= OUTPUT_TAIL_BYTES) {
        warnings.push({
            type: "truncated_output",
            message: `Output may be truncated (${text.length} ≥ ${OUTPUT_TAIL_BYTES} byte limit).`,
            severity: "medium",
        });
    }
    // 3. File-reference validation
    const fileRefs = extractFileReferences(text);
    const missingFiles = [];
    for (const ref of fileRefs) {
        if (!fileExistsRelative(cwd, ref)) {
            missingFiles.push(ref);
        }
    }
    // Cap to avoid flooding the UI
    const MAX_MISSING_SHOWN = 5;
    const shown = missingFiles.slice(0, MAX_MISSING_SHOWN);
    if (shown.length > 0) {
        const suffix = missingFiles.length > MAX_MISSING_SHOWN ? ` (+${missingFiles.length - MAX_MISSING_SHOWN} more)` : "";
        warnings.push({
            type: "missing_file",
            message: `References ${shown.length} non-existent file${shown.length > 1 ? "s" : ""}: ${shown.map((f) => path.basename(f)).join(", ")}${suffix}`,
            severity: "medium",
        });
    }
    // 4. Self-contradiction heuristics
    warnings.push(...checkSelfContradiction(text));
    // Compute confidence score
    let confidence = 1.0;
    for (const w of warnings) {
        switch (w.severity) {
            case "high":
                confidence -= 0.3;
                break;
            case "medium":
                confidence -= 0.15;
                break;
            case "low":
                confidence -= 0.05;
                break;
        }
    }
    confidence = Math.max(0, Math.min(1, Math.round(confidence * 100) / 100));
    return { passed: warnings.length === 0, warnings, confidence };
}
function getPiInvocation(args) {
    // Use the current Node.js executable with the CLI script
    const currentScript = process.argv[1];
    const scriptExt = path.extname(currentScript).toLowerCase();
    // If running via bun or the script is a .ts file, use the hamr binary
    const isBun = process.execPath.includes("bun") || process.execPath.includes("$bunfs");
    if (isBun) {
        // Try the locally built cli.js first, fall back to global hamr
        const repoCli = path.resolve(import.meta.dirname ?? __dirname, "../../../dist/cli.js");
        if (fs.existsSync(repoCli)) {
            return { command: process.execPath, args: ["run", repoCli, ...args] };
        }
        return { command: "hamr", args };
    }
    // Node.js: use the current script if it exists and is a .js file
    if (currentScript && !currentScript.startsWith("/$bunfs") && scriptExt === ".js") {
        if (fs.existsSync(currentScript)) {
            return { command: process.execPath, args: [currentScript, ...args] };
        }
    }
    return { command: "hamr", args };
}
/**
 * Bash-only fast path: spawn /bin/bash -c <task> directly.
 * No agent loop, no locks, no model calls. ~50ms startup.
 */
async function runBashFastPath(workerId, task, cwd, signal, onEvent) {
    onEvent({ type: "bash_fast_path_start", task });
    let wasAborted = false;
    let stdout = "";
    let stderr = "";
    const exitCode = await new Promise((resolve) => {
        const proc = spawn("/bin/bash", ["-c", task], {
            cwd,
            shell: false,
            stdio: ["ignore", "pipe", "pipe"],
            env: { ...process.env },
            detached: process.platform !== "win32",
        });
        if (proc.pid)
            trackDetachedChildPid(proc.pid);
        proc.stdout.on("data", (data) => {
            stdout += data.toString();
        });
        proc.stderr.on("data", (data) => {
            stderr += data.toString();
        });
        proc.on("close", (code) => {
            if (proc.pid)
                untrackDetachedChildPid(proc.pid);
            resolve(code ?? 0);
        });
        proc.on("error", () => resolve(1));
        if (signal) {
            const killProc = () => {
                wasAborted = true;
                if (proc.pid)
                    killProcessTree(proc.pid);
                setTimeout(() => {
                    if (!proc.killed)
                        proc.kill("SIGKILL");
                }, 5000);
            };
            if (signal.aborted) {
                killProc();
            }
            else {
                signal.addEventListener("abort", killProc, { once: true });
                proc.on("close", () => {
                    if (proc.pid)
                        untrackDetachedChildPid(proc.pid);
                    signal.removeEventListener("abort", killProc);
                });
            }
        }
    });
    if (wasAborted) {
        return { status: "aborted", workerId, task, reason: "user" };
    }
    onEvent({ type: "bash_fast_path_end", exitCode, stdoutPreview: stdout.slice(0, 1024) });
    if (exitCode !== 0) {
        const outcome = {
            status: "failed",
            workerId,
            task,
            error: stderr || `exit code ${exitCode}`,
            text: stdout,
        };
        const validation = validateWorkerOutput(outcome, cwd);
        return { ...outcome, validation };
    }
    const validation = validateWorkerOutput({ status: "done", workerId, task, text: stdout, usage: { ...EMPTY_USAGE } }, cwd);
    return {
        status: "done",
        workerId,
        task,
        text: stdout,
        usage: { ...EMPTY_USAGE },
        estimatedUsage: true,
        validation,
    };
}
async function runWorkerChildProcess(workerId, task, cwd, signal, onEvent, workerModel, workerTools, parentConfig) {
    // ─── Bash-only fast path ───────────────────────────────────────────────
    if (isBashFastPathTools(workerTools)) {
        return runBashFastPath(workerId, task, cwd, signal, onEvent);
    }
    const args = ["--mode", "json", "-p", "--no-session"];
    if (workerModel)
        args.push("--model", workerModel);
    if (workerTools && workerTools.length > 0)
        args.push("--tools", workerTools.join(","));
    args.push(task);
    // ─── Serialize parent config for child process ────────────────────────
    // Written with 0o600 (then chmod'd, since `mode` is masked by umask) because it
    // carries the provider API key and CF-Access credentials. Registered for
    // cleanup on parent exit so a crash can't orphan the secret in /tmp.
    let childConfigPath;
    if (parentConfig) {
        childConfigPath = path.join(os.tmpdir(), `hamr-config-${randomUUID()}.json`);
        try {
            fs.writeFileSync(childConfigPath, JSON.stringify(parentConfig), { encoding: "utf-8", mode: 0o600 });
            fs.chmodSync(childConfigPath, 0o600);
            registerOrphanedConfigForCleanup(childConfigPath);
        }
        catch {
            // If we can't write the config, the child falls back to normal startup.
            childConfigPath = undefined;
        }
    }
    let wasAborted = false;
    let stderr = "";
    let outputText = "";
    let usage = { ...EMPTY_USAGE };
    let model;
    let estimatedUsage = true;
    let stopReason;
    const exitCode = await new Promise((resolve) => {
        const invocation = getPiInvocation(args);
        const childEnv = {
            ...Object.fromEntries(Object.entries(process.env).filter((entry) => entry[1] !== undefined)),
            [ENV_TREE_REMAINING]: String(treeBudgetRemaining),
        };
        if (childConfigPath) {
            childEnv[ENV_CHILD_CONFIG] = childConfigPath;
        }
        const proc = spawn(invocation.command, invocation.args, {
            cwd,
            shell: false,
            stdio: ["ignore", "pipe", "pipe"],
            env: childEnv,
            detached: process.platform !== "win32",
        });
        if (proc.pid)
            trackDetachedChildPid(proc.pid);
        let buffer = "";
        const processLine = (line) => {
            if (!line.trim())
                return;
            let event;
            try {
                event = JSON.parse(line);
            }
            catch {
                return;
            }
            onEvent(event);
            if (event.type === "message_end" && event.message) {
                const msg = event.message;
                if (msg.role === "assistant") {
                    const msgUsage = msg.usage;
                    if (msgUsage) {
                        usage = { ...EMPTY_USAGE, ...msgUsage };
                        estimatedUsage = false;
                    }
                    if (msg.model && typeof msg.model === "string")
                        model = msg.model;
                    if (msg.stopReason && typeof msg.stopReason === "string")
                        stopReason = msg.stopReason;
                    // Extract text
                    const content = msg.content;
                    if (content) {
                        for (const part of content) {
                            if (part.type === "text" && part.text)
                                outputText += part.text;
                        }
                    }
                }
            }
            if (event.type === "tool_result_end" && event.message) {
                onEvent({ ...event, _recorded: true });
            }
        };
        proc.stdout.on("data", (data) => {
            buffer += data.toString();
            const lines = buffer.split("\n");
            buffer = lines.pop() || "";
            for (const line of lines)
                processLine(line);
        });
        proc.stderr.on("data", (data) => {
            stderr += data.toString();
        });
        proc.on("close", (code) => {
            if (buffer.trim())
                processLine(buffer);
            // Clean up the temp config file after the child exits.
            if (childConfigPath) {
                unregisterOrphanedConfigForCleanup(childConfigPath);
                try {
                    fs.unlinkSync(childConfigPath);
                }
                catch {
                    /* best-effort */
                }
            }
            resolve(code ?? 0);
        });
        proc.on("error", () => {
            if (childConfigPath) {
                unregisterOrphanedConfigForCleanup(childConfigPath);
                try {
                    fs.unlinkSync(childConfigPath);
                }
                catch {
                    /* best-effort */
                }
            }
            resolve(1);
        });
        if (signal) {
            const killProc = () => {
                wasAborted = true;
                if (proc.pid)
                    killProcessTree(proc.pid);
                setTimeout(() => {
                    if (!proc.killed)
                        proc.kill("SIGKILL");
                }, 5000);
            };
            if (signal.aborted) {
                killProc();
            }
            else {
                signal.addEventListener("abort", killProc, { once: true });
                // Clean up the abort listener when the process exits normally,
                // preventing stale `killProc` closures from accumulating on the
                // abort signal across many swarm calls (memory leak).
                proc.on("close", () => {
                    if (proc.pid)
                        untrackDetachedChildPid(proc.pid);
                    signal.removeEventListener("abort", killProc);
                });
            }
        }
    });
    if (wasAborted) {
        return {
            status: "aborted",
            workerId,
            task,
            reason: "user",
        };
    }
    if (exitCode !== 0 || stderr) {
        return {
            status: "failed",
            workerId,
            task,
            error: stderr || `exit code ${exitCode}`,
            text: outputText,
        };
    }
    // Success — build the done outcome
    const outcome = {
        status: "done",
        workerId,
        task,
        text: outputText,
        usage,
    };
    if (model)
        outcome.model = model;
    if (estimatedUsage !== undefined)
        outcome.estimatedUsage = estimatedUsage;
    if (stopReason)
        outcome.stopReason = stopReason;
    return outcome;
}
// ─── Concurrency limiter ─────────────────────────────────────────────────────
async function mapWithConcurrencyLimit(items, concurrency, fn, onProgress) {
    if (items.length === 0)
        return [];
    const limit = Math.max(1, Math.min(concurrency, items.length));
    const results = new Array(items.length);
    let nextIndex = 0;
    let completed = 0;
    const worker = async () => {
        while (true) {
            const idx = nextIndex++;
            if (idx >= items.length)
                return;
            results[idx] = await fn(items[idx], idx);
            completed++;
            onProgress(completed);
        }
    };
    const workers = Array.from({ length: limit }, () => worker());
    await Promise.all(workers);
    return results;
}
// ─── Worker state management ─────────────────────────────────────────────────
function createWorkerState(workerId, task, cwd, logPath) {
    return {
        workerId,
        taskPreview: task.length > 80 ? `${task.slice(0, 80)}…` : task,
        cwd,
        status: "queued",
        usage: { ...EMPTY_USAGE },
        recentEvents: [],
        pendingFlush: [],
        outputTail: "",
        logPath,
    };
}
function pushEvent(ws, event) {
    // One timestamp shared by the in-memory preview and the on-disk record.
    const ts = Date.now();
    // Truncated in-memory preview (ring buffer, UI only).
    const entry = {
        ts,
        type: event.type ?? "unknown",
        data: JSON.stringify(event).slice(0, 256),
    };
    ws.recentEvents.push(entry);
    if (ws.recentEvents.length > EVENTS_IN_MEMORY) {
        ws.recentEvents.splice(0, ws.recentEvents.length - EVENTS_IN_MEMORY);
    }
    const type = event.type;
    ws.lastActivity = type;
    if (type === "tool_execution_start" || type === "tool_execution_end") {
        ws.lastTool = event.toolName ?? type;
    }
    // Update output tail from streamed text (avoid O(n²) concat: build then slice).
    if (type === "message_update" || type === "message_end") {
        const msg = event.message;
        if (msg?.content) {
            let text = "";
            for (const part of msg.content) {
                if (part.type === "text" && part.text)
                    text += part.text;
            }
            if (text) {
                ws.outputTail =
                    ws.outputTail.length + text.length > OUTPUT_TAIL_BYTES
                        ? (ws.outputTail + text).slice(-OUTPUT_TAIL_BYTES)
                        : ws.outputTail + text;
            }
        }
    }
    // Incremental disk flush: store full event JSON for forensic replay.
    // Only the in-memory recentEvents ring buffer is truncated.
    ws.pendingFlush.push(`${JSON.stringify({ ts, type: event.type ?? "unknown", data: event })}\n`);
    if (ws.pendingFlush.length >= FLUSH_BATCH_SIZE) {
        flushPendingEvents(ws);
    }
}
// ─── O(1) status transitions ─────────────────────────────────────────────────
function countInit(run) {
    run._cnt = { queued: 0, running: 0, done: 0, failed: 0, aborted: 0, tok: 0 };
}
function countIncr(run, status, tokens) {
    switch (status) {
        case "queued":
            run._cnt.queued++;
            break;
        case "running":
            run._cnt.running++;
            break;
        case "done":
            run._cnt.done++;
            run._cnt.tok += tokens ?? 0;
            break;
        case "failed":
            run._cnt.failed++;
            run._cnt.tok += tokens ?? 0;
            break;
        case "aborted":
            run._cnt.aborted++;
            break;
    }
}
function countDecr(run, status, tokens) {
    switch (status) {
        case "queued":
            run._cnt.queued--;
            break;
        case "running":
            run._cnt.running--;
            break;
        case "done":
            run._cnt.done--;
            run._cnt.tok -= tokens ?? 0;
            break;
        case "failed":
            run._cnt.failed--;
            run._cnt.tok -= tokens ?? 0;
            break;
        case "aborted":
            run._cnt.aborted--;
            break;
    }
}
/** Transition a worker to a new status. O(1). */
function transition(run, ws, to, tokens) {
    countDecr(run, ws.status, ws.usage.totalTokens);
    ws.status = to;
    countIncr(run, to, tokens);
    run.queued = run._cnt.queued;
    run.running = run._cnt.running;
    run.done = run._cnt.done;
    run.failed = run._cnt.failed;
    run.aborted = run._cnt.aborted;
    run.usage = { ...run.usage, totalTokens: run._cnt.tok };
}
/** Accumulate a worker's cost into the run total. Call after transition to done/failed. */
function accumulateCost(run, usage) {
    const c = usage.cost;
    if (!c?.total)
        return;
    const prev = run.usage.cost ?? { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 };
    run.usage = {
        ...run.usage,
        cost: {
            input: (prev.input ?? 0) + (c.input ?? 0),
            output: (prev.output ?? 0) + (c.output ?? 0),
            cacheRead: (prev.cacheRead ?? 0) + (c.cacheRead ?? 0),
            cacheWrite: (prev.cacheWrite ?? 0) + (c.cacheWrite ?? 0),
            total: prev.total + c.total,
        },
    };
}
// ─── Core execution ──────────────────────────────────────────────────────────
async function executeSingleWorker(run, workerId, task, cwd, signal, _onUpdate, ctx, workerModel, workerTools, stepTimeoutMs, workerArtifact) {
    const logPath = path.join(run.logDir, "workers", `${workerId}.events.ndjson`);
    const resultPath = path.join(run.logDir, "workers", `${workerId}.final.md`);
    let ws = run.workers.get(workerId) ?? createWorkerState(workerId, task, cwd, logPath);
    if (workerModel)
        ws.model = workerModel;
    transition(run, ws, "running");
    ws.startedAt = Date.now();
    run.workers.set(workerId, ws);
    updateStatusWidget(ctx);
    // --- Incremental disk flush: flush pending events every FLUSH_INTERVAL_MS ---
    ws.flushTimer = setInterval(() => {
        const current = run.workers.get(workerId);
        if (current)
            flushPendingEvents(current);
    }, FLUSH_INTERVAL_MS);
    // --- Step timeout: per-worker AbortController that kills on expiry ---
    const effectiveStepTimeout = stepTimeoutMs ?? ENV_STEP_TIMEOUT_MS;
    const stepAbortController = new AbortController();
    const stepTimer = setTimeout(() => stepAbortController.abort(), effectiveStepTimeout);
    // Forward tool signal to step abort so user escape / session disposal also kills the worker.
    const toolSignalHandler = () => stepAbortController.abort();
    if (signal) {
        if (signal.aborted) {
            stepAbortController.abort();
        }
        else {
            signal.addEventListener("abort", toolSignalHandler, { once: true });
        }
    }
    // ─── Build parent config for child process fast-start path ──────────
    let parentConfig;
    if (ctx.model) {
        try {
            const auth = await ctx.modelRegistry.getApiKeyAndHeaders(ctx.model);
            parentConfig = {
                apiKey: auth.ok ? auth.apiKey : undefined,
                apiHeaders: auth.ok ? auth.headers : undefined,
                apiEnv: auth.ok ? auth.env : undefined,
                provider: ctx.model.provider,
                modelId: ctx.model.id,
                modelName: ctx.model.name,
                modelApi: ctx.model.api,
                modelBaseUrl: ctx.model.baseUrl,
                modelContextWindow: ctx.model.contextWindow,
                modelMaxTokens: ctx.model.maxTokens,
                modelReasoning: ctx.model.reasoning,
                modelInput: [...(ctx.model.input ?? [])],
                modelCost: ctx.model.cost ? { ...ctx.model.cost } : undefined,
                modelHeaders: ctx.model.headers ? { ...ctx.model.headers } : undefined,
                modelThinkingLevelMap: ctx.model.thinkingLevelMap ? { ...ctx.model.thinkingLevelMap } : undefined,
                modelCompat: ctx.model.compat ? safeJsonClone(ctx.model.compat) : undefined,
                // Default to parent's default active tool list if not explicitly specified by user
                toolNames: workerTools ?? ["read", "bash", "edit", "write"],
                systemPrompt: ctx.getSystemPrompt(),
                cwd: ctx.cwd,
                treeBudgetRemaining,
            };
        }
        catch {
            // Auth resolution failed — child will fall back to normal startup.
        }
    }
    try {
        const result = await runWorkerChildProcess(workerId, task, cwd, stepAbortController.signal, (event) => {
            ws = run.workers.get(workerId) ?? ws;
            pushEvent(ws, event);
        }, workerModel, workerTools, parentConfig);
        ws = run.workers.get(workerId) ?? ws;
        ws.endedAt = Date.now();
        if (result.status === "done") {
            ws.model = result.model ?? ws.model;
            ws.stopReason = result.stopReason;
        }
        // If the step timer (not the tool signal) fired, remap aborted → timeout.
        if (result.status === "aborted" && stepAbortController.signal.aborted && !signal?.aborted) {
            flushWorkerLog(ws);
            run.workers.set(workerId, ws);
            transition(run, ws, "aborted");
            updateStatusWidget(ctx);
            return {
                status: "timeout",
                workerId,
                task,
                partialText: ws.outputTail.slice(-OUTPUT_TAIL_BYTES),
            };
        }
        if (result.status === "aborted") {
            flushWorkerLog(ws);
            run.workers.set(workerId, ws);
            transition(run, ws, "aborted");
            updateStatusWidget(ctx);
            return result;
        }
        if (result.status === "failed") {
            ws.errorMessage = result.error;
            ws.finalOutput = result.text.slice(0, OUTPUT_TAIL_BYTES);
            ws.resultPath = resultPath;
            try {
                fs.writeFileSync(resultPath, result.text.slice(0, OUTPUT_TAIL_BYTES), { encoding: "utf-8", mode: 0o600 });
            }
            catch {
                /* best-effort */
            }
            flushWorkerLog(ws);
            run.workers.set(workerId, ws);
            transition(run, ws, "failed", ws.usage.totalTokens);
            accumulateCost(run, ws.usage);
            updateStatusWidget(ctx);
            const validation = validateWorkerOutput(result, cwd);
            return { ...result, validation };
        }
        // Success (or empty output — still counts as done)
        if (result.status === "timeout") {
            flushWorkerLog(ws);
            run.workers.set(workerId, ws);
            transition(run, ws, "aborted");
            updateStatusWidget(ctx);
            const validation = validateWorkerOutput(result, cwd);
            return { ...result, validation };
        }
        // At this point result.status has been narrowed to "done" since
        // "aborted", "failed", and "timeout" returned early above.
        if (result.status !== "done") {
            flushWorkerLog(ws);
            run.workers.set(workerId, ws);
            transition(run, ws, "aborted");
            updateStatusWidget(ctx);
            return result;
        }
        // Require a non-empty final assistant text.  If the worker only produced
        // thinking events (thinking_start / thinking_delta / thinking_end) but no
        // final assistant message, treat it as a failure.
        if (!result.text || result.text.trim().length === 0) {
            ws.errorMessage = "Worker produced no final assistant text (only thinking events or empty response).";
            ws.endedAt = Date.now();
            flushWorkerLog(ws);
            run.workers.set(workerId, ws);
            transition(run, ws, "failed", ws.usage.totalTokens);
            accumulateCost(run, ws.usage);
            updateStatusWidget(ctx);
            return {
                status: "failed",
                workerId,
                task,
                text: "",
                error: ws.errorMessage,
                validation: validateWorkerOutput(result, cwd),
            };
        }
        // Validate artifact contract: if the task declares an output artifact path,
        // the file must exist and be non-empty.
        if (workerArtifact) {
            const resolved = path.resolve(cwd, workerArtifact);
            try {
                if (!fs.existsSync(resolved)) {
                    const err = `Artifact contract not met: required output file "${workerArtifact}" does not exist.`;
                    ws.errorMessage = err;
                    ws.endedAt = Date.now();
                    flushWorkerLog(ws);
                    run.workers.set(workerId, ws);
                    transition(run, ws, "failed", ws.usage.totalTokens);
                    accumulateCost(run, ws.usage);
                    updateStatusWidget(ctx);
                    return { status: "failed", workerId, task, text: result.text, error: err };
                }
                if (fs.statSync(resolved).size === 0) {
                    const err = `Artifact contract not met: required output file "${workerArtifact}" is empty.`;
                    ws.errorMessage = err;
                    ws.endedAt = Date.now();
                    flushWorkerLog(ws);
                    run.workers.set(workerId, ws);
                    transition(run, ws, "failed", ws.usage.totalTokens);
                    accumulateCost(run, ws.usage);
                    updateStatusWidget(ctx);
                    return { status: "failed", workerId, task, text: result.text, error: err };
                }
            }
            catch (err) {
                const errMsg = `Artifact contract check failed for "${workerArtifact}": ${err instanceof Error ? err.message : String(err)}`;
                ws.errorMessage = errMsg;
                ws.endedAt = Date.now();
                flushWorkerLog(ws);
                run.workers.set(workerId, ws);
                transition(run, ws, "failed", ws.usage.totalTokens);
                accumulateCost(run, ws.usage);
                updateStatusWidget(ctx);
                return { status: "failed", workerId, task, text: result.text, error: errMsg };
            }
        }
        ws.usage = result.usage;
        ws.estimatedUsage = result.estimatedUsage ?? false;
        // In-memory preview stays capped (one per worker); the full output is
        // persisted to resultPath on disk below.
        ws.finalOutput = result.text.slice(0, OUTPUT_TAIL_BYTES);
        ws.resultPath = resultPath;
        try {
            fs.writeFileSync(resultPath, result.text, { encoding: "utf-8", mode: 0o600 });
        }
        catch {
            /* best-effort */
        }
        flushWorkerLog(ws);
        run.workers.set(workerId, ws);
        transition(run, ws, "done", ws.usage.totalTokens);
        accumulateCost(run, ws.usage);
        updateStatusWidget(ctx);
        const validation = validateWorkerOutput(result, cwd);
        return {
            status: "done",
            workerId,
            task,
            text: result.text,
            usage: ws.usage,
            model: ws.model ?? result.model,
            estimatedUsage: ws.estimatedUsage,
            stopReason: result.stopReason,
            validation,
        };
    }
    catch (error) {
        ws = run.workers.get(workerId) ?? ws;
        ws.errorMessage = error instanceof Error ? error.message : String(error);
        ws.endedAt = Date.now();
        flushWorkerLog(ws);
        run.workers.set(workerId, ws);
        transition(run, ws, "failed", ws.usage.totalTokens);
        accumulateCost(run, ws.usage);
        updateStatusWidget(ctx);
        return { status: "failed", workerId, task, text: "", error: ws.errorMessage };
    }
    finally {
        clearTimeout(stepTimer);
        if (signal)
            signal.removeEventListener("abort", toolSignalHandler);
        if (ws.flushTimer) {
            clearInterval(ws.flushTimer);
            ws.flushTimer = undefined;
        }
    }
}
// ─── Mode: Tasks (parallel batch) ────────────────────────────────────────────
async function executeTasks(run, taskItems, concurrency, failFast, signal, onUpdate, ctx, stepTimeoutMs) {
    // Initialize all workers as queued — O(1) counter init
    const N = taskItems.length;
    for (let i = 0; i < N; i++) {
        const item = taskItems[i];
        run.workers.set(padWorkerId(i, N), createWorkerState(padWorkerId(i, N), item.task, item.cwd ?? ctx.cwd, path.join(run.logDir, "workers", `${padWorkerId(i, N)}.events.ndjson`)));
    }
    run._cnt.queued = N;
    run.queued = N;
    run.total = N;
    updateStatusWidget(ctx);
    const results = await mapWithConcurrencyLimit(taskItems, concurrency, async (item, idx) => {
        if (signal?.aborted) {
            const wid = padWorkerId(idx, N);
            const w = run.workers.get(wid);
            if (w)
                transition(run, w, "aborted");
            return { status: "aborted", workerId: wid, task: item.task, reason: "parent" };
        }
        return executeSingleWorker(run, padWorkerId(idx, N), item.task, item.cwd ?? ctx.cwd, signal, onUpdate, ctx, item.model, item.tools, stepTimeoutMs, item.artifact);
    }, (done) => {
        if (onUpdate) {
            onUpdate({ text: `${done}/${N} tasks complete`, details: { mode: "tasks", runId: run.runId, done, total: N } });
        }
    });
    // failFast: abort remaining queued/running
    if (failFast) {
        const firstFailure = results.find((r) => r.status === "failed");
        if (firstFailure) {
            for (const ws of run.workers.values()) {
                if (ws.status === "queued" || ws.status === "running") {
                    transition(run, ws, "aborted");
                    ws.endedAt = Date.now();
                }
            }
            updateStatusWidget(ctx);
        }
    }
    run.endedAt = Date.now();
    updateStatusWidget(ctx);
    return results;
}
// ─── Mode: Chain (serial) ────────────────────────────────────────────────────
async function executeChain(run, chainItems, failFast, signal, onUpdate, ctx, stepTimeoutMs) {
    const N = chainItems.length;
    for (let i = 0; i < N; i++) {
        const item = chainItems[i];
        run.workers.set(padWorkerId(i, N), createWorkerState(padWorkerId(i, N), item.task, item.cwd ?? ctx.cwd, path.join(run.logDir, "workers", `${padWorkerId(i, N)}.events.ndjson`)));
    }
    run._cnt.queued = N;
    run.queued = N;
    run.total = N;
    updateStatusWidget(ctx);
    const results = [];
    let previousOutput = "";
    for (let i = 0; i < N; i++) {
        if (signal?.aborted) {
            abortRemaining(run, i, N);
            updateStatusWidget(ctx);
            results.push({ status: "aborted", workerId: padWorkerId(i, N), task: chainItems[i].task, reason: "parent" });
            break;
        }
        const item = chainItems[i];
        const workerId = padWorkerId(i, N);
        const taskWithContext = item.task.replace(/\{previous\}/g, previousOutput);
        // Create a per-step AbortController so that the abort listener attached
        // inside runWorkerChildProcess is scoped to this step only. The tool
        // signal (user escape / dispose only) is forwarded: if the user escapes,
        // the step is killed. The listener is cleaned up after each step,
        // preventing stale closures from accumulating on the tool signal
        // across long chain runs.
        const stepController = new AbortController();
        let toolSignalListener;
        if (signal) {
            if (signal.aborted) {
                stepController.abort();
            }
            else {
                toolSignalListener = () => stepController.abort();
                signal.addEventListener("abort", toolSignalListener, { once: true });
            }
        }
        let result;
        try {
            result = await executeSingleWorker(run, workerId, taskWithContext, item.cwd ?? ctx.cwd, stepController.signal, onUpdate, ctx, item.model, item.tools, stepTimeoutMs, item.artifact);
        }
        finally {
            // Always clean up the tool signal listener for this step.
            if (toolSignalListener && signal) {
                signal.removeEventListener("abort", toolSignalListener);
            }
        }
        results.push(result);
        if (onUpdate) {
            onUpdate({
                text: `${i + 1}/${N} chain steps complete`,
                details: { mode: "chain", runId: run.runId, step: i + 1, total: N },
            });
        }
        if (failFast && result.status === "failed") {
            abortRemaining(run, i + 1, N);
            updateStatusWidget(ctx);
            break;
        }
        // If the tool signal fired during this step (step was aborted), stop
        // the chain — the user (or session) requested cancellation.
        if (signal?.aborted) {
            abortRemaining(run, i + 1, N);
            updateStatusWidget(ctx);
            break;
        }
        // Extract output text for {previous} placeholder in the next step.
        // Only done/failed outcomes carry meaningful text; aborted/timeout are
        // stopped by the checks above before reaching here.
        if (result.status === "done" || result.status === "failed") {
            previousOutput = result.text;
        }
        else if (result.status === "timeout") {
            previousOutput = result.partialText;
        }
    }
    run.endedAt = Date.now();
    updateStatusWidget(ctx);
    return results;
}
/** Transition all workers from idx to N to "aborted". */
function abortRemaining(run, fromIdx, total) {
    for (let j = fromIdx; j < total; j++) {
        const w = run.workers.get(padWorkerId(j, total));
        if (w && (w.status === "queued" || w.status === "running")) {
            transition(run, w, "aborted");
            w.endedAt = Date.now();
        }
    }
}
async function executeStages(run, stages, concurrency, failFast, signal, onUpdate, ctx, stepTimeoutMs) {
    // O(1) counter init: all workers start queued
    let globalIdx = 0;
    const totalTasks = stages.reduce((sum, s) => sum + s.tasks.length, 0);
    for (const stage of stages) {
        for (const item of stage.tasks) {
            run.workers.set(padWorkerId(globalIdx, totalTasks), createWorkerState(padWorkerId(globalIdx, totalTasks), item.task, item.cwd ?? ctx.cwd, path.join(run.logDir, "workers", `${padWorkerId(globalIdx, totalTasks)}.events.ndjson`)));
            globalIdx++;
        }
    }
    run._cnt.queued = totalTasks;
    run.queued = totalTasks;
    run.total = totalTasks;
    run.mode = "stages";
    updateStatusWidget(ctx);
    const allResults = [];
    let stageOffset = 0;
    let previousOutput = "";
    for (let si = 0; si < stages.length; si++) {
        if (signal?.aborted)
            break;
        const stage = stages[si];
        if (stage.mode === "parallel") {
            const stageResults = await executeTasks(run, stage.tasks, concurrency, failFast, signal, undefined, ctx, stepTimeoutMs);
            allResults.push(...stageResults);
            if (failFast && stageResults.some((r) => r.status === "failed")) {
                for (const ws of run.workers.values()) {
                    if (ws.status === "queued") {
                        transition(run, ws, "aborted");
                        ws.endedAt = Date.now();
                    }
                }
                updateStatusWidget(ctx);
                break;
            }
            const lastResult = stageResults[stageResults.length - 1];
            if (lastResult) {
                if (lastResult.status === "done" || lastResult.status === "failed") {
                    previousOutput = lastResult.text;
                }
                else if (lastResult.status === "timeout") {
                    previousOutput = lastResult.partialText;
                }
            }
        }
        else {
            // chain within stage
            for (let i = 0; i < stage.tasks.length; i++) {
                if (signal?.aborted)
                    break;
                const item = stage.tasks[i];
                const taskWithContext = item.task.replace(/\{previous\}/g, previousOutput);
                const workerId = padWorkerId(stageOffset + i, totalTasks);
                const result = await executeSingleWorker(run, workerId, taskWithContext, item.cwd ?? ctx.cwd, signal, undefined, ctx, item.model, item.tools, stepTimeoutMs, item.artifact);
                allResults.push(result);
                if (failFast && result.status === "failed") {
                    for (const ws of run.workers.values()) {
                        if (ws.status === "queued") {
                            transition(run, ws, "aborted");
                            ws.endedAt = Date.now();
                        }
                    }
                    break;
                }
                if (result.status === "done" || result.status === "failed") {
                    previousOutput = result.text;
                }
                else if (result.status === "timeout") {
                    previousOutput = result.partialText;
                }
            }
        }
        stageOffset += stage.tasks.length;
        if (onUpdate) {
            onUpdate({
                text: `Stage ${si + 1}/${stages.length} complete`,
                details: { mode: "stages", runId: run.runId, stage: si + 1, totalStages: stages.length },
            });
        }
    }
    run.endedAt = Date.now();
    updateStatusWidget(ctx);
    return allResults;
}
// ─── Tool registration ───────────────────────────────────────────────────────
const TaskItem = Type.Object({
    task: Type.String({ description: "Focused, self-contained task for one worker subagent." }),
    cwd: Type.Optional(Type.String({ description: "Working directory for this worker." })),
    model: Type.Optional(Type.String({ description: "Model override for this worker (e.g. claude-haiku-4-5)." })),
    tools: Type.Optional(Type.Array(Type.String(), { description: 'Restrict tools for this worker (e.g. ["read", "grep"]).' })),
    artifact: Type.Optional(Type.String({
        description: "Path to an output file the worker must produce. The runner validates it exists and is non-empty after completion.",
    })),
});
const SubagentParams = Type.Object({
    // Legacy: serial subtasks (kept for backward compatibility)
    subtasks: Type.Optional(Type.Array(TaskItem, {
        description: "DEPRECATED. Use 'tasks' (parallel) or 'chain' (sequential) instead. One or more subtasks run in order.",
        minItems: 1,
    })),
    // Parallel batch
    tasks: Type.Optional(Type.Array(TaskItem, {
        description: "Batch of tasks run in parallel with bounded concurrency. Each worker gets an isolated child process.",
        minItems: 1,
    })),
    // Sequential chain
    chain: Type.Optional(Type.Array(TaskItem, {
        description: "Tasks run sequentially in order. Use {previous} in a task to reference the output of the prior step.",
        minItems: 1,
    })),
    // Mixed stages
    stages: Type.Optional(Type.Array(Type.Object({
        mode: Type.Union([Type.Literal("parallel"), Type.Literal("chain")], {
            description: 'Stage execution mode: "parallel" for concurrent workers, "chain" for sequential.',
        }),
        tasks: Type.Array(TaskItem, { description: "Tasks for this stage.", minItems: 1 }),
    }), {
        description: "Array of stages executed sequentially. Each stage runs its tasks in the specified mode.",
        minItems: 1,
    })),
    concurrency: Type.Optional(Type.Number({ description: `Max concurrent workers (default: ${ENV_MAX_CONCURRENCY}). Clamped to safe limits.` })),
    failFast: Type.Optional(Type.Boolean({
        description: "If true, abort remaining workers on first failure (default: false).",
        default: false,
    })),
    observe: Type.Optional(Type.Union([Type.Literal("silent"), Type.Literal("compact"), Type.Literal("verbose")], {
        description: "Observation verbosity: silent (no per-worker output), compact (summary only), verbose (per-worker details).",
        default: "compact",
    })),
    stepTimeoutMs: Type.Optional(Type.Number({
        description: `Per-worker timeout in ms (default: ${ENV_STEP_TIMEOUT_MS} = ${ENV_STEP_TIMEOUT_MS / 60000}min). Set HAMR_SUBAGENT_STEP_TIMEOUT_MS env var to change default.`,
    })),
    totalTimeoutMs: Type.Optional(Type.Number({
        description: `Per-run total timeout in ms (default: ${ENV_TOTAL_TIMEOUT_MS} = ${ENV_TOTAL_TIMEOUT_MS / 60000}min). Set HAMR_SUBAGENT_TOTAL_TIMEOUT_MS env var to change default.`,
    })),
});
function modeDescription() {
    return [
        "Delegate focused subtasks to parallel or sequential worker subagents.",
        "Each worker runs as an isolated child hamr process.",
        "",
        "Modes (exactly one required):",
        `• tasks: parallel batch with bounded concurrency (default max ${ENV_MAX_CONCURRENCY}).`,
        "• chain: sequential execution in order. Use {previous} in a task to reference prior output.",
        "• stages: sequential stages; each stage can be 'parallel' or 'chain'.",
        "• subtasks: DEPRECATED serial alias — use 'chain' instead.",
        "",
        "Concurrency is capped for memory/GPU safety. Thousands of planned workers are allowed;",
        "hundreds of simultaneous model calls are not. Default concurrency is conservative.",
        "",
        `A global budget (default ${ENV_TOTAL_BUDGET}) caps total subagents across recursive calls. Set HAMR_SUBAGENT_BUDGET env var to adjust.`,
        "",
        `Each worker has a step timeout (default ${ENV_STEP_TIMEOUT_MS / 60000}min) and a per-run total timeout (default ${ENV_TOTAL_TIMEOUT_MS / 60000}min).`,
        "Set HAMR_SUBAGENT_STEP_TIMEOUT_MS and HAMR_SUBAGENT_TOTAL_TIMEOUT_MS env vars to change defaults.",
        "",
        "Workers that fail do not kill the swarm unless failFast=true.",
        "Full logs persisted to disk: .hamr/subagents/runs/<runId>/",
    ].join("\n");
}
function registerSubagentTool(pi) {
    pi.registerTool(defineTool({
        name: "delegate_subagents",
        label: "Subagents",
        description: modeDescription(),
        promptSnippet: "Use delegate_subagents to dispatch focused subtasks to parallel/sequential worker subagents.",
        promptGuidelines: [
            "Each task should be a clearly scoped, self-contained piece of work.",
            "For independent subtasks, use 'tasks' (parallel batch). For dependent steps, use 'chain' or 'stages'.",
            "Use {previous} in chain/stages tasks to reference the prior worker's output.",
            "Parallel concurrency is bounded — do not worry about overloading, the system caps it safely.",
            "Delegate only as many tasks as the work genuinely warrants.",
        ],
        parameters: SubagentParams,
        renderCall: (args, theme, context) => {
            const hasSubtasks = (args.subtasks?.length ?? 0) > 0;
            const hasTasks = (args.tasks?.length ?? 0) > 0;
            const hasChain = (args.chain?.length ?? 0) > 0;
            const hasStages = (args.stages?.length ?? 0) > 0;
            let modeLabel;
            let count;
            let items;
            if (hasStages) {
                const stageList = args.stages;
                count = stageList.reduce((s, st) => s + st.tasks.length, 0);
                modeLabel = `stages (${stageList.length} stages, ${count} tasks)`;
                items = stageList.flatMap((s) => s.tasks);
            }
            else if (hasTasks) {
                modeLabel = `parallel (${args.tasks.length} tasks)`;
                count = args.tasks.length;
                items = args.tasks;
            }
            else if (hasChain) {
                modeLabel = `chain (${args.chain.length} steps)`;
                count = args.chain.length;
                items = args.chain;
            }
            else {
                modeLabel = `serial (${args.subtasks?.length ?? 0} tasks)`;
                items = args.subtasks ?? [];
                count = items.length;
            }
            const displayCount = context.expanded ? items.length : Math.min(items.length, 3);
            let text = theme.fg("toolTitle", theme.bold("delegate_subagents ")) + theme.fg("accent", modeLabel);
            for (let i = 0; i < displayCount; i++) {
                const itemTools = items[i]?.tools;
                const isFastPath = isBashFastPathTools(itemTools);
                const modeIndicator = isFastPath ? theme.fg("success", " ⚡bash") : "";
                const preview = items[i].task.length > 80 ? `${items[i].task.slice(0, 80)}…` : items[i].task;
                text += `\n  ${theme.fg("muted", `${i + 1}.`)} ${theme.fg("dim", preview)}${modeIndicator}`;
            }
            if (!context.expanded && items.length > 3)
                text += `\n  ${theme.fg("muted", `… +${items.length - 3} more`)}`;
            const stepMs = args.stepTimeoutMs ?? ENV_STEP_TIMEOUT_MS;
            const totalMs = args.totalTimeoutMs ?? ENV_TOTAL_TIMEOUT_MS;
            text += `\n  ${theme.fg("muted", `timeouts: step ${Math.round(stepMs / 1000)}s / total ${Math.round(totalMs / 60000)}min`)}`;
            return new Text(text, 0, 0);
        },
        renderResult: (result, options, theme) => {
            const details = result.details;
            if (!details?.results) {
                const text = result.content?.[0];
                return new Text(text?.type === "text" ? text.text : "(no output)", 0, 0);
            }
            const { results, mode: dMode, runId, logDir, done, failed, aborted, timedOut: timedOutCount } = details;
            const successCount = results.filter((r) => r.status === "done").length;
            const failCount = results.filter((r) => r.status === "failed").length;
            const abortedCount = aborted ?? results.filter((r) => r.status === "aborted").length;
            const timeoutCount = timedOutCount ?? results.filter((r) => r.status === "timeout").length;
            // Aggregate usage across all workers for display
            const agg = { tok: 0, cost: 0 };
            for (const r of results) {
                if (r.status === "done") {
                    agg.tok += r.usage.totalTokens ?? 0;
                    agg.cost += r.usage.cost?.total ?? 0;
                }
            }
            const aggLine = agg.tok > 0 ? `Total: ↓${formatTokens(agg.tok)} tok${agg.cost > 0 ? ` · $${agg.cost.toFixed(4)}` : ""}` : "";
            if (!options.expanded) {
                // Collapsed: summary line + log path
                const statusParts = [];
                if (done)
                    statusParts.push(`${done ?? results.length} done`);
                if (failCount > 0)
                    statusParts.push(`${failCount} failed`);
                if (abortedCount > 0)
                    statusParts.push(`${abortedCount} aborted`);
                if (timeoutCount > 0)
                    statusParts.push(`${timeoutCount} timed out`);
                // Count validation warnings across all outcomes
                let validationWarningsTotal = 0;
                let validationHighCount = 0;
                for (const r of results) {
                    const v = r.validation;
                    if (v) {
                        validationWarningsTotal += v.warnings.length;
                        validationHighCount += v.warnings.filter((w) => w.severity === "high").length;
                    }
                }
                let text = theme.fg("toolTitle", `${dMode} `) + theme.fg("accent", statusParts.join(", "));
                if (aggLine)
                    text += `\n${theme.fg("dim", aggLine)}`;
                if (validationWarningsTotal > 0) {
                    if (validationHighCount > 0) {
                        // Surface high-severity warnings inline; don't hide behind Ctrl+O
                        const highWarnings = [];
                        for (const r of results) {
                            const v = r.validation;
                            if (v) {
                                for (const w of v.warnings) {
                                    if (w.severity === "high")
                                        highWarnings.push(`[${r.workerId}] ${w.message}`);
                                }
                            }
                        }
                        text += `\n${theme.fg("warning", `⚠ ${validationHighCount} critical output warning${validationHighCount > 1 ? "s" : ""}:`)}`;
                        for (const hw of highWarnings.slice(0, 3)) {
                            text += `\n  ${theme.fg("warning", hw)}`;
                        }
                        if (highWarnings.length > 3) {
                            text += `\n  ${theme.fg("muted", `… +${highWarnings.length - 3} more (expand to review)`)}`;
                        }
                        if (validationWarningsTotal > validationHighCount) {
                            text += `\n  ${theme.fg("muted", `+${validationWarningsTotal - validationHighCount} lower-severity warnings (expand to review)`)}`;
                        }
                    }
                    else {
                        text += `\n${theme.fg("muted", `⚡ ${validationWarningsTotal} output warning${validationWarningsTotal > 1 ? "s" : ""} (expand to review)`)}`;
                    }
                }
                text += `\n${theme.fg("muted", `logs: ${logDir}`)}`;
                text += `\n${theme.fg("toolTitle", "▸ Press Ctrl+O to expand per-worker details")}`;
                return new Text(text, 0, 0);
            }
            // Expanded: per-worker details with Markdown + formatted tool calls
            const mdTheme = getMarkdownTheme();
            const fg = theme.fg.bind(theme);
            const container = new Container();
            const headerIcon = failCount > 0 ? fg("warning", "◐") : fg("success", "✓");
            container.addChild(new Text(`${headerIcon} ${theme.fg("toolTitle", theme.bold(dMode))} ${theme.fg("accent", `${successCount}/${results.length} succeeded`)}`, 0, 0));
            // Show top failures first (max 5)
            const failures = results
                .map((r, i) => ({ result: r, idx: i }))
                .filter(({ result }) => result.status === "failed")
                .slice(0, 5);
            if (failures.length > 0) {
                container.addChild(new Spacer(1));
                container.addChild(new Text(fg("error", "Failures:"), 0, 0));
                for (const { result: r, idx } of failures) {
                    const f = r;
                    container.addChild(new Text(`  ${fg("error", "✕")} [${padWorkerId(idx, results.length)}] ${fg("dim", f.task.slice(0, 60))}`, 0, 0));
                    container.addChild(new Text(`    ${fg("error", f.error.slice(0, 120))}`, 0, 0));
                }
            }
            // Show recent successful workers (max 10)
            const successWorkers = results
                .map((r, i) => ({ result: r, idx: i }))
                .filter(({ result }) => result.status === "done")
                .slice(-10);
            if (successWorkers.length > 0) {
                container.addChild(new Spacer(1));
                for (const [si, { result: r, idx }] of successWorkers.entries()) {
                    // TypeScript narrows: only done outcomes pass the filter
                    const done = r;
                    const usageStr = done.usage.totalTokens ? ` ↓${formatTokens(done.usage.totalTokens)} tok` : "";
                    const modelStr = done.model ? ` ${fg("muted", done.model)}` : "";
                    container.addChild(new Text(`  ${fg("success", "✓")} [${padWorkerId(idx, results.length)}] ${fg("dim", done.task.slice(0, 80))}${modelStr}${fg("muted", usageStr)}`, 0, 0));
                    if (done.validation && done.validation.warnings.length > 0) {
                        for (const w of done.validation.warnings) {
                            const icon = w.severity === "high" ? "⚠" : w.severity === "medium" ? "⚡" : "·";
                            const color = w.severity === "high" ? "warning" : w.severity === "medium" ? "warning" : "muted";
                            container.addChild(new Text(`    ${fg(color, `${icon} ${w.message}`)}`, 0, 0));
                        }
                    }
                    if (done.text) {
                        container.addChild(new Spacer(1));
                        container.addChild(new Markdown(done.text.slice(0, 4096), 3, 0, mdTheme));
                    }
                    if (si < successWorkers.length - 1)
                        container.addChild(new Spacer(1));
                }
            }
            // Timed-out workers
            if (timeoutCount > 0) {
                container.addChild(new Spacer(1));
                container.addChild(new Text(fg("warning", `${timeoutCount} timed out`), 0, 0));
                for (const { result: r, idx } of results
                    .map((r, i) => ({ result: r, idx: i }))
                    .filter(({ result }) => result.status === "timeout")) {
                    const t = r;
                    container.addChild(new Text(`  ${fg("warning", "⏱")} [${padWorkerId(idx, results.length)}] ${fg("dim", t.task.slice(0, 60))}`, 0, 0));
                }
            }
            // Aborted workers
            if (abortedCount > 0) {
                container.addChild(new Spacer(1));
                container.addChild(new Text(fg("muted", `${abortedCount} aborted`), 0, 0));
            }
            if (aggLine) {
                container.addChild(new Spacer(1));
                container.addChild(new Text(fg("dim", aggLine), 0, 0));
            }
            container.addChild(new Spacer(1));
            container.addChild(new Text(fg("muted", `Full logs: ${logDir}`), 0, 0));
            return container;
        },
        execute: async (_toolCallId, params, signal, onUpdate, ctx) => {
            // Validate exactly one mode
            const hasSubtasks = (params.subtasks?.length ?? 0) > 0;
            const hasTasks = (params.tasks?.length ?? 0) > 0;
            const hasChain = (params.chain?.length ?? 0) > 0;
            const hasStages = (params.stages?.length ?? 0) > 0;
            const modeCount = Number(hasSubtasks) + Number(hasTasks) + Number(hasChain) + Number(hasStages);
            if (modeCount === 0) {
                return {
                    content: [
                        {
                            type: "text",
                            text: "No mode specified. Provide exactly one of: tasks, chain, stages, or subtasks (deprecated).",
                        },
                    ],
                    details: {},
                };
            }
            if (modeCount > 1) {
                return {
                    content: [
                        {
                            type: "text",
                            text: "Multiple modes specified. Provide exactly one of: tasks, chain, stages, or subtasks (deprecated).",
                        },
                    ],
                    details: {},
                };
            }
            // Determine concurrency: cloud providers get ENV_MAX_CONCURRENCY (64), local/relay capped at 1
            const config = loadHamrStartupConfig(ctx.cwd);
            const isCloud = ctx.model?.provider ? isCloudProvider(config, ctx.model.provider) : true;
            // Block non-cloud providers (relay, local endpoints) from dispatching subagents.
            // Relay models cannot inherit the parent's model config because the child
            // config serialization depends on cloud-provider auth flows, and relay
            // endpoints serve dynamic model lists that may not include the parent's model.
            if (!isCloud) {
                return {
                    content: [
                        {
                            type: "text",
                            text: `Relay/local provider "${ctx.model?.provider}" cannot dispatch subagents. ` +
                                "Switch to a cloud provider (Anthropic, OpenAI, etc.) or set `cloud = true` " +
                                "for this provider in your .hamr.toml to use delegate_subagents.",
                        },
                    ],
                    details: {},
                };
            }
            const maxConcurrency = ENV_MAX_CONCURRENCY;
            const concurrency = clamp(params.concurrency ?? ENV_MAX_CONCURRENCY, 1, maxConcurrency);
            const failFast = params.failFast ?? false;
            const observe = params.observe ?? "compact";
            // Validate task counts against ENV soft limit (warn but don't block)
            let taskCount = 0;
            if (hasSubtasks)
                taskCount = params.subtasks.length;
            else if (hasTasks)
                taskCount = params.tasks.length;
            else if (hasChain)
                taskCount = params.chain.length;
            else if (hasStages) {
                taskCount = params.stages.reduce((s, st) => s + st.tasks.length, 0);
            }
            if (taskCount > ENV_MAX_TASKS) {
                return {
                    content: [
                        {
                            type: "text",
                            text: `Too many tasks (${taskCount}). Soft limit is ${ENV_MAX_TASKS}. Set HAMR_SUBAGENT_MAX_TASKS to increase (hard max: ${ENV_HARD_MAX_TASKS}).`,
                        },
                    ],
                    details: {},
                };
            }
            if (taskCount > ENV_HARD_MAX_TASKS) {
                return {
                    content: [{ type: "text", text: `Too many tasks (${taskCount}). Hard limit is ${ENV_HARD_MAX_TASKS}.` }],
                    details: {},
                };
            }
            // In-memory budget check for this process's subtree.
            // Each process tracks its own remaining budget independently;
            // children inherit their slice via HAMR_SUBAGENT_TREE_REMAINING env.
            if (taskCount > 0) {
                if (treeBudgetRemaining < taskCount) {
                    return {
                        content: [
                            {
                                type: "text",
                                text: `Subagent budget exhausted. Only ${treeBudgetRemaining} slots remain, ${taskCount} requested. Total tree budget: ${ENV_TOTAL_BUDGET}. Wait for active subagents to complete, or set HAMR_SUBAGENT_BUDGET to increase.`,
                            },
                        ],
                        details: { budgetExhausted: true, treeBudget: ENV_TOTAL_BUDGET, remaining: treeBudgetRemaining },
                        isError: true,
                    };
                }
                treeBudgetRemaining -= taskCount;
            }
            // ─── Record spawn point in the parent session tree ──────────
            // This creates a real child node in the session tree so subagent
            // runs are walkable/inspectable, not orphan in-memory sessions.
            const spawnPolicy = {
                contextInheritance: "scoped",
                executionMode: "background",
                mergePolicy: "handoff-only",
            };
            const childSessionId = `subagent-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
            let spawnPointEntryId;
            try {
                // Use the shared session-dir encoder so the child session path matches
                // exactly what the session manager persists (honoring a configured
                // agent dir), letting the session tree cross-reference child sessions.
                const childSessionPath = getDefaultSessionDirPath(ctx.cwd);
                spawnPointEntryId = ctx.sessionManager.appendSpawnPoint(childSessionId, spawnPolicy, childSessionPath, undefined, // runId — will be set below
                "subagent swarm");
            }
            catch {
                // Best-effort: if the session manager doesn't persist (--no-session),
                // appendSpawnPoint still works on the in-memory tree.
            }
            // Create run state
            const runId = nextRunId();
            const logDir = ensureLogDir(runId, ctx.cwd);
            const run = {
                runId,
                mode: hasTasks ? "parallel" : hasChain ? "chain" : hasStages ? "stages" : "single",
                total: 0,
                queued: 0,
                running: 0,
                done: 0,
                failed: 0,
                aborted: 0,
                startedAt: Date.now(),
                usage: { ...EMPTY_USAGE },
                logDir,
                workers: new Map(),
                _cnt: { queued: 0, running: 0, done: 0, failed: 0, aborted: 0, tok: 0 },
            };
            activeRuns.set(runId, run);
            // Save run metadata
            try {
                fs.writeFileSync(path.join(logDir, "run.json"), JSON.stringify({
                    runId,
                    mode: run.mode,
                    startedAt: new Date(run.startedAt).toISOString(),
                    cwd: ctx.cwd,
                    parentSessionId: ctx.sessionManager.getSessionId(),
                }, null, 2), { encoding: "utf-8", mode: 0o600 });
            }
            catch {
                /* best-effort */
            }
            const onUpdateWrapper = observe !== "silent" && onUpdate
                ? (update) => {
                    onUpdate({
                        content: [{ type: "text", text: update.text }],
                        details: { ...update.details },
                    });
                }
                : undefined;
            const stepTimeoutMs = params.stepTimeoutMs ?? ENV_STEP_TIMEOUT_MS;
            const totalTimeoutMs = params.totalTimeoutMs ?? ENV_TOTAL_TIMEOUT_MS;
            // Total-timeout controller: when it fires, all workers are killed.
            const totalAbortController = new AbortController();
            const totalTimer = setTimeout(() => totalAbortController.abort(), totalTimeoutMs);
            // Forward the tool signal (user escape / session dispose) so either
            // source aborts the run. Internal lifecycle events (compaction, auto-retry)
            // do NOT fire this signal, so subagents continue through those.
            const forwardToolSignal = () => totalAbortController.abort();
            if (signal) {
                if (signal.aborted) {
                    totalAbortController.abort();
                }
                else {
                    signal.addEventListener("abort", forwardToolSignal, { once: true });
                }
            }
            let results;
            try {
                if (hasStages) {
                    const stageSpecs = params.stages.map((s) => ({
                        mode: s.mode,
                        tasks: s.tasks,
                    }));
                    results = await executeStages(run, stageSpecs, concurrency, failFast, totalAbortController.signal, onUpdateWrapper, ctx, stepTimeoutMs);
                }
                else if (hasTasks) {
                    const tasks = params.tasks;
                    results = await executeTasks(run, tasks, concurrency, failFast, totalAbortController.signal, onUpdateWrapper, ctx, stepTimeoutMs);
                }
                else if (hasChain) {
                    const chain = params.chain;
                    results = await executeChain(run, chain, failFast, totalAbortController.signal, onUpdateWrapper, ctx, stepTimeoutMs);
                }
                else {
                    // Legacy subtasks — run as chain
                    const subtasks = params.subtasks;
                    results = await executeChain(run, subtasks, failFast, totalAbortController.signal, onUpdateWrapper, ctx, stepTimeoutMs);
                }
            }
            catch (err) {
                // Refund budget slots for workers that never spawned, so a run that
                // aborts before/early during spawning doesn't permanently leak the
                // tree budget. Workers that did start consumed their slots via the
                // HAMR_SUBAGENT_TREE_REMAINING env passed at spawn time.
                const spawned = run.workers.size;
                if (taskCount > spawned)
                    treeBudgetRemaining += taskCount - spawned;
                throw err;
            }
            finally {
                // Save final run state
                try {
                    fs.writeFileSync(path.join(logDir, "run.json"), JSON.stringify({
                        runId,
                        mode: run.mode,
                        total: run.total,
                        done: run.done,
                        failed: run.failed,
                        aborted: run.aborted,
                        startedAt: new Date(run.startedAt).toISOString(),
                        endedAt: new Date().toISOString(),
                        usage: run.usage,
                        cwd: ctx.cwd,
                        parentSessionId: ctx.sessionManager.getSessionId(),
                    }, null, 2), { encoding: "utf-8", mode: 0o600 });
                }
                catch {
                    /* best-effort */
                }
                evictOldRuns();
                clearTimeout(totalTimer);
                if (signal)
                    signal.removeEventListener("abort", forwardToolSignal);
            }
            const errors = results.filter((r) => r.status === "failed");
            const aborted = results.filter((r) => r.status === "aborted");
            const timedOut = results.filter((r) => r.status === "timeout");
            const successCount = results.filter((r) => r.status === "done").length;
            // ─── Merge handoff into the parent session tree ────────────
            // This creates a custom_message entry as a child of the spawn
            // point, injecting subagent output into LLM context when the
            // parent continues.
            if (spawnPointEntryId) {
                try {
                    const mergeParts = [];
                    mergeParts.push(`## Subagent swarm ${runId} results`);
                    mergeParts.push(`Mode: ${run.mode}, ${successCount}/${results.length} succeeded`);
                    for (const r of results) {
                        if (r.status === "done") {
                            const done = r;
                            mergeParts.push(`### [${r.workerId}] ✓ ${done.task.slice(0, 80)}`);
                            if (done.text)
                                mergeParts.push(done.text.slice(0, 4096));
                            if (done.usage.totalTokens) {
                                mergeParts.push(`_(tokens: ${done.usage.totalTokens}, model: ${done.model ?? "unknown"})_`);
                            }
                        }
                        else if (r.status === "failed") {
                            const failed = r;
                            mergeParts.push(`### [${r.workerId}] ✕ ${failed.task.slice(0, 80)}`);
                            mergeParts.push(`Error: ${failed.error.slice(0, 500)}`);
                        }
                    }
                    ctx.sessionManager.mergeHandoff(spawnPointEntryId, "subagent_handoff", mergeParts.join("\n\n"), {
                        runId,
                        mode: run.mode,
                        total: run.total,
                        done: run.done,
                        failed: run.failed,
                        aborted: run.aborted,
                        totalTokens: run.usage.totalTokens,
                        cost: run.usage.cost,
                    });
                }
                catch {
                    // Best-effort: if merge fails, the tool result still contains
                    // the summary text for the LLM.
                }
            }
            // Build summary
            const summaryParts = [];
            const parts = [`${successCount}/${results.length} succeeded`];
            if (errors.length > 0)
                parts.push(`${errors.length} failed`);
            if (aborted.length > 0)
                parts.push(`${aborted.length} aborted`);
            if (timedOut.length > 0)
                parts.push(`${timedOut.length} timed out`);
            summaryParts.push(`Swarm ${runId} complete: ${parts.join(", ")}.`);
            if (errors.length > 0) {
                summaryParts.push("Top failures:");
                for (const r of errors.slice(0, 5)) {
                    const e = r;
                    summaryParts.push(`- [${e.workerId}] ${e.task.slice(0, 60)}: ${e.error.slice(0, 100)}`);
                }
            }
            summaryParts.push(`\nFull logs: ${logDir}`);
            summaryParts.push(`(Use /subagents open ${runId} for interactive details when available)`);
            return {
                content: [{ type: "text", text: summaryParts.join("\n") }],
                details: {
                    mode: run.mode,
                    runId,
                    total: run.total,
                    done: run.done,
                    failed: run.failed,
                    aborted: run.aborted,
                    timedOut: timedOut.length,
                    logDir,
                    results,
                },
                // Mark as error if any explicit failures, or if ALL completed workers
                // produced empty output (no final assistant text).
                ...(errors.length > 0 ||
                    (results.length > 0 &&
                        results.every((r) => {
                            const v = r.validation;
                            return v?.warnings?.some((w) => w.type === "empty_output");
                        }))
                    ? { isError: true }
                    : {}),
            };
        },
    }));
}
// ─── Extension factory ───────────────────────────────────────────────────────
export function createHamrSubagentsExtension(_getChildExtensions, depth = 0) {
    const factory = async (pi) => {
        // Leaf: no delegate tool, so recursion stops here.
        if (depth >= MAX_DEPTH)
            return;
        // Restore completed runs on session resume; clear on new/switch/fork.
        // session_before_switch fires on the OLD session's runtime (which is then
        // disposed), so session_tree can never fire post-switch. The resume restore
        // happens in session_start on the NEW session's runtime instead.
        pi.on("session_start", (event, ctx) => {
            activeRuns.clear();
            if (event.reason === "resume") {
                restoreRunsFromDisk(ctx.cwd, ctx.sessionManager.getSessionId());
            }
            updateStatusWidget(ctx);
        });
        pi.on("session_before_switch", (_, ctx) => {
            activeRuns.clear();
            updateStatusWidget(ctx);
        });
        pi.on("session_before_fork", (_, ctx) => {
            activeRuns.clear();
            updateStatusWidget(ctx);
        });
        registerSubagentTool(pi);
    };
    factory[HAMR_SUBAGENTS_FACTORY] = true;
    return factory;
}
// ─── Test-only exports (not part of the public API) ──────────────────────────
// Exposed so regression tests can verify correctness without spinning up
// full child hamr processes.
export const _testExports = {
    pushEvent,
    validateWorkerOutput,
    createWorkerState,
};
//# sourceMappingURL=subagents.js.map