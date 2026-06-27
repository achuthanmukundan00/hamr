import type { ExtensionFactory } from "../../core/extensions/types.ts";

// ─── Configuration ──────────────────────────────────────────────────────────

/** How many identical tool calls in a row trigger a nudge. */
const LOOP_THRESHOLD = 5;

/** Minimum time between nudges (ms). */
const COOLDOWN_MS = 15_000;

/** Max history length for loop detection window. */
const MAX_HISTORY = 12;

// ─── Identity key builders ──────────────────────────────────────────────────

/**
 * Build a stable identity string for a tool call.
 *
 * Two tool calls are "identical" if they have the same tool name AND the same
 * identity key. The key is chosen to capture semantically identical invocations
 * while permitting progressive exploration:
 *
 * - `read`: keyed by (path, offset, limit) — different offsets are NOT a loop.
 * - `edit`: keyed by (path, first-oldText-hash) — same targeted edit on the same content is a loop.
 * - `write`: keyed by (path, content-prefix) — differentiate iterative writes to the same file.
 * - `bash`: keyed by the command string itself.
 * - Other tools: keyed by full arguments.
 */
function identityKey(toolName: string, input: Record<string, unknown>): string {
	switch (toolName) {
		case "read": {
			const p = String(input.path ?? "");
			const o = String(input.offset ?? "");
			const l = String(input.limit ?? "");
			return `read:${p}:${o}:${l}`;
		}
		case "edit": {
			const p = String(input.path ?? "");
			// EditToolInput uses `edits: Array<{ oldText, newText }>` — key off the
			// first edit's oldText (most common single-edit pattern) and truncate to
			// avoid blowing up history with giant oldText strings.
			const edits = input.edits;
			if (Array.isArray(edits) && edits.length > 0) {
				const first = edits[0] as Record<string, unknown>;
				const o = String(first.oldText ?? "").slice(0, 120);
				return `edit:${p}:${o}`;
			}
			// Fallback for legacy format { path, oldText, newText } at top level.
			const legacy = String(input.oldText ?? "").slice(0, 120);
			return `edit:${p}:${legacy}`;
		}
		case "write": {
			const p = String(input.path ?? "");
			// Include a content prefix so iterative writes with different content
			// don't trigger a false loop nudge.
			const c = String(input.content ?? "").slice(0, 80);
			return `write:${p}:${c}`;
		}
		case "bash":
			return `bash:${String(input.command ?? "")}`;
		default:
			return `${toolName}:${JSON.stringify(input)}`;
	}
}

// ─── Extension ──────────────────────────────────────────────────────────────

export const hamrReadLoopGuardExtension: ExtensionFactory = (pi) => {
	/** Rolling history of tool call identities (oldest → newest). */
	const history: string[] = [];
	let lastNudge = 0;

	pi.on("tool_call", (event, ctx) => {
		const key = identityKey(event.toolName, event.input);

		history.push(key);
		if (history.length > MAX_HISTORY) {
			history.shift();
		}

		// Check: are the last LOOP_THRESHOLD entries all identical to the current call?
		if (history.length >= LOOP_THRESHOLD) {
			const recent = history.slice(-LOOP_THRESHOLD);
			const allSame = recent.every((k) => k === key);

			if (allSame) {
				const now = Date.now();
				if (now - lastNudge > COOLDOWN_MS) {
					lastNudge = now;
					if (!ctx.isIdle()) {
						pi.sendUserMessage(
							`(You've called "${event.toolName}" with identical arguments ${LOOP_THRESHOLD} times in a row. ` +
								`This looks like a loop — try a different approach.)`,
							{ deliverAs: "steer" },
						);
					}
				}
			}
		}
	});
};
