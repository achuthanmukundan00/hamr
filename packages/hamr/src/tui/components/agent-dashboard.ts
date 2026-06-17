import type { Component } from "sexy-tui-rs";
import { truncateToWidth, visibleWidth } from "sexy-tui-rs";
import type { HamrTheme } from "../theme/hamr-theme.js";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface AgentInfo {
	id: string;
	name: string;
	status: "running" | "waiting" | "done" | "failed";
	action?: string;
	elapsed?: number;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function formatElapsed(seconds: number): string {
	if (seconds < 60) return `${seconds}s`;
	const m = Math.floor(seconds / 60);
	const s = seconds % 60;
	return `${m}:${s.toString().padStart(2, "0")}`;
}

const STATUS_GLYPH: Record<AgentInfo["status"], { glyph: string; color: string }> = {
	running: { glyph: "◌", color: "accent" },
	waiting: { glyph: "◷", color: "muted" },
	done: { glyph: "◆", color: "success" },
	failed: { glyph: "✕", color: "error" },
};

/** Truncate a dashboard row to fit within `maxWidth` columns. */
function truncateRow(row: string, maxWidth: number): string {
	if (maxWidth <= 0) return "";
	if (visibleWidth(row) <= maxWidth) return row;
	return truncateToWidth(row, maxWidth);
}

// ─── AgentDashboard Component ────────────────────────────────────────────────

export class AgentDashboard implements Component {
	private agents: AgentInfo[] = [];
	private selectedIndex: number = 0;
	private theme: HamrTheme;

	onRetryAgent?: (index: number) => void;
	onFocusAgent?: (index: number) => void;
	onViewDetails?: (index: number) => void;
	onClose?: () => void;

	constructor(theme: HamrTheme) {
		this.theme = theme;
	}

	setTheme(theme: HamrTheme): void {
		this.theme = theme;
	}

	setAgents(agents: AgentInfo[]): void {
		this.agents = agents;
		if (this.selectedIndex >= agents.length) {
			this.selectedIndex = Math.max(0, agents.length - 1);
		}
	}

	getSelectedIndex(): number {
		return this.selectedIndex;
	}

	moveUp(): void {
		if (this.agents.length === 0) return;
		this.selectedIndex = Math.max(0, this.selectedIndex - 1);
	}

	moveDown(): void {
		if (this.agents.length === 0) return;
		this.selectedIndex = Math.min(this.agents.length - 1, this.selectedIndex + 1);
	}

	retryAgent(index: number): void {
		const agent = this.agents[index];
		if (agent) {
			agent.status = "running";
			agent.action = undefined;
			this.onRetryAgent?.(index);
		}
	}

	focusAgent(index: number): void {
		this.onFocusAgent?.(index);
	}

	viewDetails(index: number): void {
		this.onViewDetails?.(index);
	}

	isEmpty(): boolean {
		return this.agents.length === 0;
	}

	invalidate(): void {
		// no cache to clear
	}

	handleInput(data: string): void {
		// ↑ or k
		if (data === "\x1b[A" || data === "k") {
			this.moveUp();
		}
		// ↓ or j
		if (data === "\x1b[B" || data === "j") {
			this.moveDown();
		}
		// r — retry
		if (data === "r") {
			this.retryAgent(this.selectedIndex);
		}
		// enter — focus
		if (data === "\r" || data === "\n") {
			this.focusAgent(this.selectedIndex);
		}
		// d — details
		if (data === "d") {
			this.viewDetails(this.selectedIndex);
		}
		// esc — close
		if (data === "\x1b") {
			this.onClose?.();
		}
	}

	render(width: number): string[] {
		const lines: string[] = [];

		if (this.agents.length === 0) {
			const emptyLine = "  ◇  No subagents to view";
			const hint = this.theme.fg("dim", "  subagents appear when parallel tasks are dispatched");
			lines.push("");
			lines.push(" ".repeat(Math.max(0, Math.floor((width - 24) / 2))) + this.theme.fg("muted", "── subagents ──"));
			lines.push("");
			lines.push(
				" ".repeat(Math.max(0, Math.floor((width - emptyLine.length) / 2))) + this.theme.fg("muted", emptyLine),
			);
			lines.push(" ".repeat(Math.max(0, Math.floor((width - hint.length) / 2))) + hint);
			lines.push("");
			return lines;
		}

		// Header
		const header = `AGENTS · ${this.agents.length} total`;
		const pad = Math.max(0, Math.floor((width - header.length) / 2));
		lines.push("");
		lines.push(" ".repeat(pad) + this.theme.fg("accent", header));
		lines.push(this.theme.fg("border", "─".repeat(width)));

		// Agent rows
		for (let i = 0; i < this.agents.length; i++) {
			const agent = this.agents[i];
			const isSelected = i === this.selectedIndex;
			const prefix = isSelected ? this.theme.fg("accent", "▸ ") : "  ";
			const statusInfo = STATUS_GLYPH[agent.status];
			const glyph = this.theme.fg(statusInfo.color as any, statusInfo.glyph);
			const name = this.theme.fg("text", agent.name);
			const action = agent.action ? this.theme.fg("dim", `  ${agent.action}`) : "";
			const elapsed = agent.elapsed !== undefined ? this.theme.fg("muted", `  ${formatElapsed(agent.elapsed)}`) : "";

			// Truncate action to fit terminal width, preserving name + elapsed.
			const prefixAndName = `${prefix}${glyph} ${name}`;
			const suffix = elapsed;
			const availForAction = Math.max(10, width - visibleWidth(prefixAndName) - visibleWidth(suffix));
			const truncatedAction = visibleWidth(action) > availForAction ? truncateToWidth(action, availForAction) : action;
			const row = `${prefixAndName}${truncatedAction}${suffix}`;
			lines.push(truncateRow(row, width));
		}

		// Footer controls
		lines.push("");
		lines.push(this.theme.fg("dim", "  ↑↓ navigate · enter focus · d details · r retry · esc close"));

		return lines;
	}
}
