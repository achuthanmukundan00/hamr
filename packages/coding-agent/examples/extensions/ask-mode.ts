/**
 * Ask-First Mode Extension
 *
 * Toggle a safe analysis-and-proposal mode where the LLM cannot make
 * destructive changes. When active, edit/write are blocked at the
 * tool-call level and dangerous bash commands require confirmation.
 *
 * Features:
 * - Ctrl+Shift+A shortcut or /ask command to toggle
 * - Real guardrails: blocks edit/write, gates dangerous bash
 * - Compact indicator: "⚠ ask-first" at normal width, "!" below 40 cols
 * - Widget placed below editor so visible at all terminal widths
 * - State persists across /reload via appendEntry
 *
 * Usage:
 *   hamr -e ./ask-mode.ts
 * Or place in ~/.hamr/agent/extensions/ for auto-load.
 */

import type { ExtensionAPI, ExtensionContext } from "@hamr/coding-agent";
import { Key } from "@hamr/tui";

// ---------------------------------------------------------------------------
// Bash patterns that are too dangerous for ask-first mode
// ---------------------------------------------------------------------------
const DANGEROUS_BASH_PATTERNS = [
	// File deletion
	/\brm\b/,
	// File moves/renames that could be destructive
	/\bmv\b/,
	// Permission changes
	/\bchmod\b/,
	/\bchown\b/,
	// File overwrite via redirect (> file, >> file)
	/>\s*\S/,
	/>>\s*\S/,
	// Package installs
	/\b(npm|yarn|pnpm|bun)\s+(install|add|i)\b/,
	/\bpip\s+install\b/,
	/\bcargo\s+install\b/,
	/\bgo\s+install\b/,
	// Git destructive
	/\bgit\s+(commit|push|rebase|reset|clean)\b/,
	/\bgit\s+(branch\s+-[dD]|tag\s+-d)\b/,
	// Disk ops
	/\bdd\b/,
	/\bmkfs\b/,
	// System changes
	/\bsudo\b/,
	/\bsystemctl\b/,
	/\bservice\b/,
	// Network install
	/\bcurl\b.*\|\s*(ba)?sh/,
	/\bwget\b.*\|\s*(ba)?sh/,
];

function isDangerousBash(command: string): boolean {
	return DANGEROUS_BASH_PATTERNS.some((p) => p.test(command));
}

// ---------------------------------------------------------------------------
// Extension
// ---------------------------------------------------------------------------

export default function askModeExtension(pi: ExtensionAPI): void {
	let askFirst = false;

	// ── UI indicator: widget below editor, adapts to narrow terminals ──
	function updateUI(ctx: ExtensionContext): void {
		if (askFirst) {
			ctx.ui.setWidget(
				"ask-mode",
				(_tui, theme) => ({
					render(width: number): string[] {
						if (width < 40) return [theme.fg("warning", "⚠")];
						return [theme.fg("warning", "⚠ ask-first")];
					},
					invalidate(): void {
						/* stateless — width driven */
					},
				}),
				{ placement: "belowEditor" },
			);
			// Also set footer status for double visibility on wide terminals
			ctx.ui.setStatus("ask-mode", ctx.ui.theme.fg("warning", "⚠"));
		} else {
			ctx.ui.setWidget("ask-mode", undefined);
			ctx.ui.setStatus("ask-mode", undefined);
		}
	}

	function persist(): void {
		pi.appendEntry("ask-mode-state", { askFirst });
	}

	// ── Toggle ──
	function toggle(ctx: ExtensionContext): void {
		askFirst = !askFirst;
		updateUI(ctx);
		persist();
	}

	// ── Ctrl+Shift+A shortcut ──
	pi.registerShortcut(Key.ctrlShift("a"), {
		description: "Toggle ask-first mode",
		handler: async (ctx) => {
			toggle(ctx);
			ctx.ui.notify(
				askFirst ? "Ask-first: ON — reads only, edits blocked" : "Ask-first: OFF — full access restored",
				askFirst ? "warning" : "info",
			);
		},
	});

	// ── /ask command ──
	pi.registerCommand("ask", {
		description: "Toggle ask-first mode (safe analysis, no edits)",
		handler: async (_args, ctx) => {
			toggle(ctx);
			ctx.ui.notify(askFirst ? "Ask-first mode: ON" : "Ask-first mode: OFF", "info");
		},
	});

	// ── Real guardrails: block destructive tool calls ──
	pi.on("tool_call", async (event, ctx) => {
		if (!askFirst) return;

		// Block all write operations
		if (event.toolName === "write") {
			const path = event.input.path as string;
			return {
				block: true,
				reason: `Ask-first mode active. "write ${path}" blocked. Use /ask to disable ask-first first.`,
			};
		}

		// Block all edit operations
		if (event.toolName === "edit") {
			const path = event.input.path as string;
			return {
				block: true,
				reason: `Ask-first mode active. "edit ${path}" blocked. Use /ask to disable ask-first first.`,
			};
		}

		// Block dangerous bash commands
		if (event.toolName === "bash") {
			const command = (event.input.command as string) || "";
			if (isDangerousBash(command)) {
				if (ctx.hasUI) {
					ctx.ui.notify(`Ask-first: blocked dangerous command`, "warning");
				}
				return {
					block: true,
					reason: `Ask-first mode active. Dangerous bash blocked.\nCommand: ${command}\nUse /ask to disable ask-first first.`,
				};
			}
		}
	});

	// ── Restore state on session start / reload ──
	pi.on("session_start", async (_event, ctx) => {
		const entries = ctx.sessionManager.getEntries();
		const stateEntry = [...entries]
			.reverse()
			.find(
				(e): e is { type: "custom"; customType: "ask-mode-state"; data?: { askFirst: boolean } } =>
					e.type === "custom" && e.customType === "ask-mode-state",
			);
		if (stateEntry?.data?.askFirst) {
			askFirst = true;
			updateUI(ctx);
		}
	});
}
