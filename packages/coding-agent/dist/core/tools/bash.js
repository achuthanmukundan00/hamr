import { constants } from "node:fs";
import { access as fsAccess } from "node:fs/promises";
import { Container, Text, truncateToWidth } from "@hamr/tui";
import { spawn } from "child_process";
import { Type } from "typebox";
import { createDiffComponent, looksLikeUnifiedDiff } from "../../modes/interactive/components/diff.js";
import { keyHint } from "../../modes/interactive/components/keybinding-hints.js";
import { truncateToVisualLines } from "../../modes/interactive/components/visual-truncate.js";
import { theme } from "../../modes/interactive/theme/theme.js";
import { waitForChildProcess } from "../../utils/child-process.js";
import { getShellConfig, getShellEnv, killProcessTree, trackDetachedChildPid, untrackDetachedChildPid, } from "../../utils/shell.js";
import { OutputAccumulator } from "./output-accumulator.js";
import { getTextOutput, invalidArgText, str } from "./render-utils.js";
import { wrapToolDefinition } from "./tool-definition-wrapper.js";
import { DEFAULT_MAX_BYTES, DEFAULT_MAX_LINES, formatSize } from "./truncate.js";
const bashSchema = Type.Object({
    command: Type.String({ description: "Bash command to execute" }),
    timeout: Type.Optional(Type.Number({ description: "Timeout in seconds (optional, no default timeout)" })),
});
/**
 * Create bash operations using pi's built-in local shell execution backend.
 *
 * This is useful for extensions that intercept user_bash and still want pi's
 * standard local shell behavior while wrapping or rewriting commands.
 */
export function createLocalBashOperations(options) {
    return {
        exec: async (command, cwd, { onData, signal, timeout, env }) => {
            const { shell, args } = getShellConfig(options?.shellPath);
            try {
                await fsAccess(cwd, constants.F_OK);
            }
            catch {
                throw new Error(`Working directory does not exist: ${cwd}\nCannot execute bash commands.`);
            }
            if (signal?.aborted) {
                throw new Error("aborted");
            }
            const child = spawn(shell, [...args, command], {
                cwd,
                detached: process.platform !== "win32",
                env: env ?? getShellEnv(),
                stdio: ["ignore", "pipe", "pipe"],
                windowsHide: true,
            });
            if (child.pid)
                trackDetachedChildPid(child.pid);
            let timedOut = false;
            let timeoutHandle;
            const onAbort = () => {
                if (child.pid)
                    killProcessTree(child.pid);
            };
            try {
                // Set timeout if provided.
                if (timeout !== undefined && timeout > 0) {
                    timeoutHandle = setTimeout(() => {
                        timedOut = true;
                        if (child.pid)
                            killProcessTree(child.pid);
                    }, timeout * 1000);
                }
                // Stream stdout and stderr.
                child.stdout?.on("data", onData);
                child.stderr?.on("data", onData);
                // Handle abort signal by killing the entire process tree.
                if (signal) {
                    if (signal.aborted)
                        onAbort();
                    else
                        signal.addEventListener("abort", onAbort, { once: true });
                }
                // Handle shell spawn errors and wait for the process to terminate without hanging
                // on inherited stdio handles held by detached descendants.
                const exitCode = await waitForChildProcess(child);
                if (signal?.aborted) {
                    throw new Error("aborted");
                }
                if (timedOut) {
                    throw new Error(`timeout:${timeout}`);
                }
                return { exitCode };
            }
            finally {
                if (child.pid)
                    untrackDetachedChildPid(child.pid);
                if (timeoutHandle)
                    clearTimeout(timeoutHandle);
                if (signal)
                    signal.removeEventListener("abort", onAbort);
            }
        },
    };
}
function resolveSpawnContext(command, cwd, spawnHook) {
    const baseContext = { command, cwd, env: { ...getShellEnv() } };
    return spawnHook ? spawnHook(baseContext) : baseContext;
}
const BASH_PREVIEW_LINES = 5;
const BASH_UPDATE_THROTTLE_MS = 100;
class BashResultRenderComponent extends Container {
    constructor() {
        super(...arguments);
        this.state = {
            cachedWidth: undefined,
            cachedLines: undefined,
            cachedSkipped: undefined,
        };
    }
}
function formatDuration(ms) {
    return `${(ms / 1000).toFixed(1)}s`;
}
function formatBashCall(args) {
    const command = str(args?.command);
    const timeout = args?.timeout;
    const timeoutSuffix = timeout ? theme.fg("muted", ` (timeout ${timeout}s)`) : "";
    // Only the glyph is bold — the command itself stays at normal weight so the
    // visual hierarchy reads "tool" first, then its arguments.
    const glyph = theme.bold(theme.fg("toolTitle", "$"));
    const commandDisplay = command === null ? invalidArgText(theme) : command ? theme.fg("toolTitle", command) : theme.fg("toolOutput", "...");
    return `${glyph} ${commandDisplay}${timeoutSuffix}`;
}
function rebuildBashResultRenderComponent(component, result, options, showImages, startedAt, endedAt, surroundBg) {
    const state = component.state;
    component.clear();
    let output = getTextOutput(result, showImages).trim();
    const truncation = result.details?.truncation;
    const fullOutputPath = result.details?.fullOutputPath;
    if (!options.isPartial && truncation?.truncated && fullOutputPath && output.endsWith("]")) {
        const footerStart = output.lastIndexOf("\n\n[");
        if (footerStart !== -1 && output.slice(footerStart).includes(fullOutputPath)) {
            output = output.slice(0, footerStart).trimEnd();
        }
    }
    // Git diffs (and any unified diff) reuse the same renderer as file edits:
    // syntax-highlighted code on a neutral base, with green/red background bands.
    if (output && !options.isPartial && looksLikeUnifiedDiff(output)) {
        const diff = createDiffComponent(output, { unified: true, surroundBg });
        component.addChild({
            render: (width) => {
                const allLines = diff.render(width);
                if (options.expanded || allLines.length <= BASH_PREVIEW_LINES) {
                    return ["", ...allLines];
                }
                const skipped = allLines.length - BASH_PREVIEW_LINES;
                const hint = theme.fg("muted", `... (${skipped} earlier lines,`) +
                    ` ${keyHint("app.tools.expand", "to expand")}${theme.fg("muted", ")")}`;
                return ["", truncateToWidth(hint, width, "..."), ...allLines.slice(skipped)];
            },
            invalidate: () => diff.invalidate?.(),
        });
    }
    else if (output) {
        const styledOutput = output
            .split("\n")
            .map((line) => theme.fg("toolOutput", line))
            .join("\n");
        if (options.expanded) {
            component.addChild(new Text(`\n${styledOutput}`, theme.cards.toolResultIndent, 0));
        }
        else {
            component.addChild({
                render: (width) => {
                    if (state.cachedLines === undefined || state.cachedWidth !== width) {
                        const preview = truncateToVisualLines(styledOutput, BASH_PREVIEW_LINES, width, theme.cards.toolResultIndent);
                        state.cachedLines = preview.visualLines;
                        state.cachedSkipped = preview.skippedCount;
                        state.cachedWidth = width;
                    }
                    if (state.cachedSkipped && state.cachedSkipped > 0) {
                        const hint = theme.fg("muted", `... (${state.cachedSkipped} earlier lines,`) +
                            ` ${keyHint("app.tools.expand", "to expand")}${theme.fg("muted", ")")}`;
                        return ["", truncateToWidth(hint, width, "..."), ...(state.cachedLines ?? [])];
                    }
                    return ["", ...(state.cachedLines ?? [])];
                },
                invalidate: () => {
                    state.cachedWidth = undefined;
                    state.cachedLines = undefined;
                    state.cachedSkipped = undefined;
                },
            });
        }
    }
    if (truncation?.truncated || fullOutputPath) {
        const warnings = [];
        if (fullOutputPath) {
            warnings.push(`Full output: ${fullOutputPath}`);
        }
        if (truncation?.truncated) {
            if (truncation.truncatedBy === "lines") {
                warnings.push(`Truncated: showing ${truncation.outputLines} of ${truncation.totalLines} lines`);
            }
            else {
                warnings.push(`Truncated: ${truncation.outputLines} lines shown (${formatSize(truncation.maxBytes ?? DEFAULT_MAX_BYTES)} limit)`);
            }
        }
        component.addChild(new Text(`\n${theme.fg("warning", `[${warnings.join(". ")}]`)}`, theme.cards.toolResultIndent, 0));
    }
    if (startedAt !== undefined) {
        const label = options.isPartial ? "Elapsed" : "Took";
        const endTime = endedAt ?? Date.now();
        component.addChild(new Text(`\n${theme.fg("muted", `${label} ${formatDuration(endTime - startedAt)}`)}`, theme.cards.toolResultIndent, 0));
    }
}
export function createBashToolDefinition(cwd, options) {
    const ops = options?.operations ?? createLocalBashOperations({ shellPath: options?.shellPath });
    const commandPrefix = options?.commandPrefix;
    const spawnHook = options?.spawnHook;
    return {
        name: "bash",
        label: "bash",
        description: `Execute a bash command in the current working directory. Returns stdout and stderr. Output is truncated to last ${DEFAULT_MAX_LINES} lines or ${DEFAULT_MAX_BYTES / 1024}KB (whichever is hit first). If truncated, full output is saved to a temp file. Optionally provide a timeout in seconds.`,
        promptSnippet: "Execute bash commands (ls, grep, find, etc.)",
        parameters: bashSchema,
        async execute(_toolCallId, { command, timeout }, signal, onUpdate, _ctx) {
            const resolvedCommand = commandPrefix ? `${commandPrefix}\n${command}` : command;
            const spawnContext = resolveSpawnContext(resolvedCommand, cwd, spawnHook);
            const output = new OutputAccumulator({ tempFilePrefix: "hamr-bash" });
            let acceptingOutput = true;
            let updateTimer;
            let updateDirty = false;
            let lastUpdateAt = 0;
            const emitOutputUpdate = () => {
                if (!onUpdate || !updateDirty)
                    return;
                updateDirty = false;
                lastUpdateAt = Date.now();
                const snapshot = output.snapshot({ persistIfTruncated: true });
                onUpdate({
                    content: [{ type: "text", text: snapshot.content || "" }],
                    details: {
                        truncation: snapshot.truncation.truncated ? snapshot.truncation : undefined,
                        fullOutputPath: snapshot.fullOutputPath,
                    },
                });
            };
            const clearUpdateTimer = () => {
                if (updateTimer) {
                    clearTimeout(updateTimer);
                    updateTimer = undefined;
                }
            };
            const scheduleOutputUpdate = () => {
                if (!onUpdate)
                    return;
                updateDirty = true;
                const delay = BASH_UPDATE_THROTTLE_MS - (Date.now() - lastUpdateAt);
                if (delay <= 0) {
                    clearUpdateTimer();
                    emitOutputUpdate();
                    return;
                }
                updateTimer ??= setTimeout(() => {
                    updateTimer = undefined;
                    emitOutputUpdate();
                }, delay);
            };
            if (onUpdate) {
                onUpdate({ content: [], details: undefined });
            }
            const handleData = (data) => {
                if (!acceptingOutput)
                    return;
                output.append(data);
                scheduleOutputUpdate();
            };
            const finishOutput = async () => {
                acceptingOutput = false;
                output.finish();
                clearUpdateTimer();
                emitOutputUpdate();
                const snapshot = output.snapshot({ persistIfTruncated: true });
                await output.closeTempFile();
                return snapshot;
            };
            const formatOutput = (snapshot, emptyText = "(no output)") => {
                const truncation = snapshot.truncation;
                let text = snapshot.content || emptyText;
                let details;
                if (truncation.truncated) {
                    details = { truncation, fullOutputPath: snapshot.fullOutputPath };
                    const startLine = truncation.totalLines - truncation.outputLines + 1;
                    const endLine = truncation.totalLines;
                    if (truncation.lastLinePartial) {
                        const lastLineSize = formatSize(output.getLastLineBytes());
                        text += `\n\n[Showing last ${formatSize(truncation.outputBytes)} of line ${endLine} (line is ${lastLineSize}). Full output: ${snapshot.fullOutputPath}]`;
                    }
                    else if (truncation.truncatedBy === "lines") {
                        text += `\n\n[Showing lines ${startLine}-${endLine} of ${truncation.totalLines}. Full output: ${snapshot.fullOutputPath}]`;
                    }
                    else {
                        text += `\n\n[Showing lines ${startLine}-${endLine} of ${truncation.totalLines} (${formatSize(DEFAULT_MAX_BYTES)} limit). Full output: ${snapshot.fullOutputPath}]`;
                    }
                }
                return { text, details };
            };
            const appendStatus = (text, status) => `${text ? `${text}\n\n` : ""}${status}`;
            try {
                let exitCode;
                try {
                    const result = await ops.exec(spawnContext.command, spawnContext.cwd, {
                        onData: handleData,
                        signal,
                        timeout,
                        env: spawnContext.env,
                    });
                    exitCode = result.exitCode;
                }
                catch (err) {
                    const snapshot = await finishOutput();
                    const { text } = formatOutput(snapshot, "");
                    if (err instanceof Error && err.message === "aborted") {
                        throw new Error(appendStatus(text, "Command aborted"));
                    }
                    if (err instanceof Error && err.message.startsWith("timeout:")) {
                        const timeoutSecs = err.message.split(":")[1];
                        throw new Error(appendStatus(text, `Command timed out after ${timeoutSecs} seconds`));
                    }
                    throw err;
                }
                const snapshot = await finishOutput();
                const { text: outputText, details } = formatOutput(snapshot);
                if (exitCode !== 0 && exitCode !== null) {
                    throw new Error(appendStatus(outputText, `Command exited with code ${exitCode}`));
                }
                return { content: [{ type: "text", text: outputText }], details };
            }
            finally {
                clearUpdateTimer();
            }
        },
        renderCall(args, _theme, context) {
            const state = context.state;
            if (context.executionStarted && state.startedAt === undefined) {
                state.startedAt = Date.now();
                state.endedAt = undefined;
            }
            const text = context.lastComponent ?? new Text("", theme.cards.toolIndent, 0);
            text.setText(formatBashCall(args));
            return text;
        },
        renderResult(result, options, _theme, context) {
            const state = context.state;
            if (state.startedAt !== undefined && options.isPartial && !state.interval) {
                state.interval = setInterval(() => context.invalidate(), 1000);
            }
            if (!options.isPartial || context.isError) {
                state.endedAt ??= Date.now();
                if (state.interval) {
                    clearInterval(state.interval);
                    state.interval = undefined;
                }
            }
            const component = context.lastComponent ?? new BashResultRenderComponent();
            // Match the card surface tool-execution paints behind a settled result,
            // so diff bands restore it instead of the terminal default on padding.
            const surroundBg = theme.cards.shadedSurfaces
                ? context.isError
                    ? "toolErrorBg"
                    : "toolSuccessBg"
                : undefined;
            rebuildBashResultRenderComponent(component, result, options, context.showImages, state.startedAt, state.endedAt, surroundBg);
            component.invalidate();
            return component;
        },
    };
}
export function createBashTool(cwd, options) {
    return wrapToolDefinition(createBashToolDefinition(cwd, options));
}
//# sourceMappingURL=bash.js.map