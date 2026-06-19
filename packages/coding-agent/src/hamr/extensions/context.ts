import { Text } from "@hamr/tui";
import type { ExtensionCommandContext, ExtensionFactory, MessageRenderOptions } from "../../core/extensions/types.ts";
import { formatSkillsForPrompt } from "../../core/skills.ts";
import type { CustomMessage } from "../../core/messages.ts";
import type { Theme, ThemeColor } from "../../modes/interactive/theme/theme.ts";

// ─── Token estimation ─────────────────────────────────────────────────────────

function estimateChars(text: string): number {
	return Math.ceil(text.length / 4);
}

// ─── Context breakdown ───────────────────────────────────────────────────────

export interface ContextBreakdown {
	modelName: string | undefined;
	modelId: string | undefined;
	contextWindow: number;
	tokens: number | null;
	percent: number | null;
	systemPrompt: number;
	skills: number;
	contextFiles: number;
	messagesAndTools: number | null;
}

function computeBreakdown(ctx: ExtensionCommandContext): ContextBreakdown {
	const model = ctx.model;
	const usage = ctx.getContextUsage();
	const opts = ctx.getSystemPromptOptions();
	const systemPromptText = ctx.getSystemPrompt();

	const contextWindow = usage?.contextWindow ?? model?.contextWindow ?? 0;
	const tokens = usage?.tokens ?? null;
	const percent = usage?.percent ?? null;

	// Estimate each system-prompt section by re-rendering it in isolation.
	const skillsText = formatSkillsForPrompt(opts.skills ?? []);
	const skillsTokens = estimateChars(skillsText);

	const contextFilesText =
		(opts.contextFiles ?? [])
			.map(({ path, content }) => `<project_instructions path="${path}">\n${content}\n</project_instructions>\n\n`)
			.join("");
	const contextFilesTokens = estimateChars(contextFilesText);

	const totalSystemTokens = estimateChars(systemPromptText);
	const baseSystemTokens = Math.max(0, totalSystemTokens - skillsTokens - contextFilesTokens);

	const messagesAndTools = tokens !== null ? Math.max(0, tokens - totalSystemTokens) : null;

	return {
		modelName: model?.name,
		modelId: model?.id,
		contextWindow,
		tokens,
		percent,
		systemPrompt: baseSystemTokens,
		skills: skillsTokens,
		contextFiles: contextFilesTokens,
		messagesAndTools,
	};
}

// ─── Rendering ───────────────────────────────────────────────────────────────

function fmt(n: number): string {
	if (n < 1000) return String(Math.round(n));
	if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k`;
	return `${(n / 1_000_000).toFixed(1)}m`;
}

function renderDisplay(breakdown: ContextBreakdown, theme: Theme): string {
	const { contextWindow, tokens, percent } = breakdown;

	const TOTAL_SLOTS = 25;
	const filledSlots = percent !== null ? Math.round((percent / 100) * TOTAL_SLOTS) : 0;
	const usageColor: ThemeColor = (percent ?? 0) > 90 ? "error" : (percent ?? 0) > 70 ? "warning" : "accent";

	const iconRows: string[] = [];
	for (let row = 0; row < 5; row++) {
		const icons: string[] = [];
		for (let col = 0; col < 5; col++) {
			const slot = row * 5 + col;
			icons.push(slot < filledSlots ? theme.fg(usageColor, "⛁") : theme.fg("dim", "⛶"));
		}
		iconRows.push(icons.join(" "));
	}

	const tokenStr =
		tokens !== null ? `${fmt(tokens)}/${fmt(contextWindow)} tokens` : `?/${fmt(contextWindow)} tokens`;
	const pctStr = percent !== null ? ` (${Math.round(percent)}%)` : "";
	const freeTokens = tokens !== null && contextWindow > 0 ? contextWindow - tokens : null;
	const freePct = freeTokens !== null && contextWindow > 0 ? (freeTokens / contextWindow) * 100 : null;

	const INDENT = "            ";
	const dot = (color: ThemeColor, ch: string) => theme.fg(color, ch);
	const bold = (s: string) => theme.bold(s);

	const lines: string[] = [];
	lines.push(theme.bold("Context Usage"));
	lines.push("");
	lines.push(`${iconRows[0]}    ${breakdown.modelName ?? "No model"}`);
	lines.push(`${iconRows[1]}    ${theme.fg("dim", breakdown.modelId ?? "")}`);
	lines.push(`${iconRows[2]}    ${theme.fg("dim", `${tokenStr}${pctStr}`)}`);
	lines.push(iconRows[3]);
	lines.push(`${iconRows[4]}    ${theme.fg("dim", theme.italic("Estimated usage by category"))}`);
	lines.push("");
	lines.push(`${INDENT}${dot("dim", "⛁")} ${bold("System prompt:")} ${fmt(breakdown.systemPrompt)} tokens`);
	if (breakdown.skills > 0) {
		lines.push(`${INDENT}${dot("accent", "⛁")} ${bold("Skills:")} ${fmt(breakdown.skills)} tokens`);
	}
	if (breakdown.contextFiles > 0) {
		lines.push(`${INDENT}${dot("warning", "⛁")} ${bold("Context files:")} ${fmt(breakdown.contextFiles)} tokens`);
	}
	lines.push(
		`${INDENT}${dot(usageColor, "⛁")} ${bold("Messages + tools:")} ${
			breakdown.messagesAndTools !== null ? `${fmt(breakdown.messagesAndTools)} tokens` : "unknown"
		}`,
	);
	lines.push(
		`${INDENT}${dot("dim", "⛶")} ${bold("Free space:")} ${
			freeTokens !== null ? `${fmt(freeTokens)} (${Math.round(freePct!)}%)` : "unknown"
		}`,
	);

	return lines.join("\n");
}

// ─── Extension ───────────────────────────────────────────────────────────────

export const hamrContextExtension: ExtensionFactory = (pi) => {
	pi.registerMessageRenderer(
		"hamr.context",
		(message: CustomMessage<ContextBreakdown>, _options: MessageRenderOptions, theme: Theme) => {
			if (!message.details) return undefined;
			return new Text(renderDisplay(message.details, theme), 1, 0);
		},
	);

	pi.registerCommand("context", {
		description: "Show context window usage",
		handler: async (_args: string, ctx: ExtensionCommandContext): Promise<void> => {
			const breakdown = computeBreakdown(ctx);
			pi.sendMessage({
				customType: "hamr.context",
				content: `Context: ${breakdown.tokens !== null ? fmt(breakdown.tokens) : "?"}/${fmt(breakdown.contextWindow)} tokens`,
				display: true,
				details: breakdown,
			});
		},
	});
};
