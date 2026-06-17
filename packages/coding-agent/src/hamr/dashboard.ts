import type { Theme } from "../modes/interactive/theme/theme.ts";
import { type Focusable, Key, matchesKey, visibleWidth } from "@hamr/tui";

export interface SlashCommandEntry {
  name: string;
  description?: string;
}

export interface DashboardData {
  commands: SlashCommandEntry[];
  provider: string;
  modelName: string;
  thinkingLevel: string;
  sessionFile: string;
  messageCount: number;
  contextPercent: number | null;
  contextWindow: number;
}

export class DashboardComponent implements Focusable {
  focused = false;
  private theme: Theme;
  private done: (result: undefined) => void;
  private data: DashboardData;
  private selected = 0;

  constructor(theme: Theme, done: (result: undefined) => void, data: DashboardData) {
    this.theme = theme;
    this.done = done;
    this.data = data;
  }

  handleInput(data: string): void {
    if (matchesKey(data, Key.escape)) { this.done(undefined); return; }
    if (this.data.commands.length === 0) return;
    if (matchesKey(data, Key.up)) { if (this.selected > 0) this.selected--; }
    else if (matchesKey(data, Key.down)) { if (this.selected < this.data.commands.length - 1) this.selected++; }
    else if (matchesKey(data, Key.enter)) { this.done(undefined); }
  }

  render(availableWidth: number): string[] {
    const th = this.theme;
    const innerW = availableWidth - 2;
    const d = this.data;
    const lines: string[] = [];

    const pad = (s: string, len: number): string => s + " ".repeat(Math.max(0, len - visibleWidth(s)));
    const row = (content: string): string => th.fg("border", "│") + pad(content, innerW) + th.fg("border", "│");
    const labelValue = (label: string, value: string): string => `   ${th.fg("dim", label)} ${value}`;

    lines.push(th.fg("border", `╭${"─".repeat(innerW)}╮`));
    lines.push(row(` ${th.bold(th.fg("accent", "⚡ Hamr Dashboard"))}`));
    lines.push(th.fg("border", `├${"─".repeat(innerW)}┤`));
    lines.push(row(""));

    lines.push(row(` ${th.bold(th.fg("accent", "Commands"))}`));
    lines.push(row(""));

    if (d.commands.length === 0) {
      lines.push(row(`   ${th.fg("dim", "(no commands available)")}`));
    } else {
      const maxVisible = Math.min(d.commands.length, 10);
      for (let i = 0; i < maxVisible; i++) {
        const cmd = d.commands[i]!;
        const isSelected = i === this.selected;
        const prefix = isSelected ? th.fg("accent", " ▸ ") : "   ";
        const cmdDisplay = isSelected ? th.fg("accent", `/${cmd.name}`) : `/${cmd.name}`;
        const descDisplay = cmd.description ? `  ${th.fg("dim", cmd.description)}` : "";
        lines.push(row(`${prefix}${cmdDisplay}${descDisplay}`));
      }
    }

    lines.push(row(""));
    lines.push(row(` ${th.fg("dim", "─── Model ───")}`));
    lines.push(row(""));
    lines.push(row(labelValue("Provider:", th.fg("accent", d.provider))));
    lines.push(row(labelValue("Model:", `   ${th.fg("accent", d.modelName)}`)));
    lines.push(row(labelValue("Thinking:", ` ${th.fg("accent", d.thinkingLevel)}`)));

    lines.push(row(""));
    lines.push(row(` ${th.fg("dim", "─── Session ───")}`));
    lines.push(row(""));

    const lineAvail = innerW - 18;
    const shortPath = d.sessionFile.length > lineAvail ? "..." + d.sessionFile.slice(-(lineAvail - 3)) : d.sessionFile;
    lines.push(row(labelValue("Session:", ` ${th.fg("accent", shortPath)}`)));
    lines.push(row(labelValue("Messages:", `${th.fg("accent", String(d.messageCount))}`)));

    if (d.contextPercent !== null) {
      const pct = th.fg("accent", `${d.contextPercent.toFixed(1)}%`);
      const win = th.fg("dim", ` / ${(d.contextWindow / 1000).toFixed(0)}K`);
      lines.push(row(labelValue("Context:", ` ${pct}${win}`)));
    } else {
      lines.push(row(labelValue("Context:", ` ${th.fg("accent", "N/A")}`)));
    }

    lines.push(row(""));
    lines.push(th.fg("border", `├${"─".repeat(innerW)}┤`));
    lines.push(row(` ${th.fg("dim", "↑↓ navigate  ·  Enter select  ·  Esc close")}`));
    lines.push(th.fg("border", `╰${"─".repeat(innerW)}╯`));
    return lines;
  }

  invalidate(): void {}
  dispose(): void {}
}
