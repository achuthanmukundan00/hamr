import { hasSubstantialContent, registerHamrProviders, repairLocalToolCalls } from "../repair.js";
import { RAINBOW_WORD_FRAMES } from "../shimmer.js";
import { loadHamrStartupConfig } from "../startup-config.js";
const COLD_START_TIMEOUT_MS = 5_000;
let coldStartTimer = null;
let hasReceivedContent = false;
function clearColdStartTimer() {
    if (coldStartTimer !== null) {
        clearTimeout(coldStartTimer);
        coldStartTimer = null;
    }
}
/** Condense a raw turn-error message (which may be a 502 HTML page) to one line. */
function summarizeTurnError(raw) {
    if (!raw)
        return "unknown error";
    const stripped = raw
        .replace(/<[^>]*>/g, " ")
        .replace(/\s+/g, " ")
        .trim();
    const text = stripped || raw.trim();
    return text.length > 200 ? `${text.slice(0, 197)}…` : text;
}
/**
 * Providers/relay extension: registers hamr's configured providers, repairs
 * local-model tool calls, drives the relay cold-start indicator, and surfaces
 * turn failures. This is hamr's "identity" layer — relay/local model support.
 */
export const hamrProvidersExtension = async (pi) => {
    const config = loadHamrStartupConfig(process.cwd());
    await registerHamrProviders(pi, config);
    // Cold-start detection: show a warning if a relay/local model takes >5s for
    // first content. Cloud models skip the cold-start logic entirely.
    pi.on("message_start", (_event, ctx) => {
        if (_event.message.role === "assistant") {
            hasReceivedContent = false;
            const model = ctx.model;
            const isRelay = model?.provider === "relay" || model?.api === "relay";
            if (!isRelay)
                return;
            clearColdStartTimer();
            coldStartTimer = setTimeout(() => {
                coldStartTimer = null;
                if (!hasReceivedContent) {
                    ctx.ui.setWorkingIndicator({ frames: ["Cold starting..."], intervalMs: 1000 });
                }
            }, COLD_START_TIMEOUT_MS);
        }
    });
    pi.on("message_update", (_event, ctx) => {
        if (_event.message.role !== "assistant")
            return;
        if (!hasReceivedContent && hasSubstantialContent(_event.assistantMessageEvent)) {
            hasReceivedContent = true;
            clearColdStartTimer();
            ctx.ui.setWorkingIndicator({ frames: RAINBOW_WORD_FRAMES, intervalMs: 150 });
        }
    });
    pi.on("session_shutdown", () => {
        clearColdStartTimer();
        hasReceivedContent = false;
    });
    // Repair local-model tool calls once a message completes.
    pi.on("message_end", (event, ctx) => {
        clearColdStartTimer();
        hasReceivedContent = false;
        if (event.message.role !== "assistant")
            return;
        const replacement = repairLocalToolCalls(event.message, ctx);
        return replacement ? { message: replacement } : undefined;
    });
    // Surface a display-only note when a turn fails (relay 502/timeout, terminated
    // stream, …). Does NOT inject a prompt back to the model or prefill the editor.
    pi.on("turn_end", (event, ctx) => {
        const message = event.message;
        if (message.role === "assistant" && message.stopReason === "error") {
            ctx.ui.notify(`Model request failed: ${summarizeTurnError(message.errorMessage)}`, "error");
        }
    });
};
//# sourceMappingURL=providers.js.map