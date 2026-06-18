import type { AssistantMessage } from "@hamr/ai";
import { Box, Container, Markdown, type MarkdownTheme, Spacer, Text } from "@hamr/tui";
import { getMarkdownTheme, theme } from "../theme/theme.ts";

const OSC133_ZONE_START = "\x1b]133;A\x07";
const OSC133_ZONE_END = "\x1b]133;B\x07";
const OSC133_ZONE_FINAL = "\x1b]133;C\x07";

/** Convert a hex color to an ANSI foreground escape. */
function hexToAnsiFg(hex: string): string {
	const r = parseInt(hex.slice(1, 3), 16);
	const g = parseInt(hex.slice(3, 5), 16);
	const b = parseInt(hex.slice(5, 7), 16);
	return `\x1b[38;2;${r};${g};${b}m`;
}

/** Compute a very-dark tinted background ANSI escape from a model accent hex. */
function hexToBg(hex: string): string {
	const r = Math.round(parseInt(hex.slice(1, 3), 16) * 0.12);
	const g = Math.round(parseInt(hex.slice(3, 5), 16) * 0.12);
	const b = Math.round(parseInt(hex.slice(5, 7), 16) * 0.12);
	return `\x1b[48;2;${r};${g};${b}m`;
}

/**
 * Component that renders a complete assistant message.
 *
 * Renders thinking blocks and text blocks as visually distinct cards,
 * each with the active model's brand-tinted background. The model
 * accent heading ("● Response") appears only when text content is
 * present; thinking uses its own "◌ THOUGHT" heading.
 */
export class AssistantMessageComponent extends Container {
	private contentContainer: Container;
	private hideThinkingBlock: boolean;
	private markdownTheme: MarkdownTheme;
	private hiddenThinkingLabel: string;
	private lastMessage?: AssistantMessage;
	private hasToolCalls = false;
	private modelAccent?: string;
	private modelGlyph?: string;

	constructor(
		message?: AssistantMessage,
		hideThinkingBlock = false,
		markdownTheme: MarkdownTheme = getMarkdownTheme(),
		hiddenThinkingLabel = "Thinking...",
		modelAccent?: string,
		modelGlyph?: string,
	) {
		super();

		this.hideThinkingBlock = hideThinkingBlock;
		this.markdownTheme = markdownTheme;
		this.hiddenThinkingLabel = hiddenThinkingLabel;
		this.modelAccent = modelAccent;
		this.modelGlyph = modelGlyph;

		this.contentContainer = new Container();
		this.addChild(this.contentContainer);

		if (message) {
			this.updateContent(message);
		}
	}

	override invalidate(): void {
		super.invalidate();
		if (this.lastMessage) {
			this.updateContent(this.lastMessage);
		}
	}

	setHideThinkingBlock(hide: boolean): void {
		this.hideThinkingBlock = hide;
		if (this.lastMessage) {
			this.updateContent(this.lastMessage);
		}
	}

	setHiddenThinkingLabel(label: string): void {
		this.hiddenThinkingLabel = label;
		if (this.lastMessage) {
			this.updateContent(this.lastMessage);
		}
	}

	setModelAccent(hex: string | undefined): void {
		this.modelAccent = hex;
		if (this.lastMessage) {
			this.updateContent(this.lastMessage);
		}
	}

	override render(width: number): string[] {
		const lines = super.render(width);
		if (this.hasToolCalls || lines.length === 0) {
			return lines;
		}

		lines[0] = OSC133_ZONE_START + lines[0];
		lines[lines.length - 1] = OSC133_ZONE_END + OSC133_ZONE_FINAL + lines[lines.length - 1];
		return lines;
	}

	updateContent(message: AssistantMessage): void {
		this.lastMessage = message;
		this.contentContainer.clear();

		const hasVisibleContent = message.content.some(
			(c) => (c.type === "text" && c.text.trim()) || (c.type === "thinking" && c.thinking.trim()),
		);

		if (!hasVisibleContent) {
			// Still show errors/aborted status even without content
			const hasToolCalls = message.content.some((c) => c.type === "toolCall");
			this.hasToolCalls = hasToolCalls;
			if (!hasToolCalls && message.stopReason === "aborted") {
				const msg =
					message.errorMessage && message.errorMessage !== "Request was aborted"
						? message.errorMessage
						: "Operation aborted";
				this.contentContainer.addChild(new Text(theme.fg("error", msg), 1, 0));
			} else if (!hasToolCalls && message.stopReason === "error") {
				this.contentContainer.addChild(
					new Text(theme.fg("error", `Error: ${message.errorMessage || "Unknown error"}`), 1, 0),
				);
			}
			return;
		}

		// Model accent ANI escapes (pre-computed for reuse).
		const accentFg = this.modelAccent && theme.modelAdaptive ? hexToAnsiFg(this.modelAccent) : undefined;
		const bgAnsi = this.modelAccent && theme.modelAdaptive ? hexToBg(this.modelAccent) : undefined;

		// Helper: model accent foreground color for headings.
		const accent = (s: string) => (accentFg ? `${accentFg}${s}\x1b[39m` : theme.fg("accent", s));

		// Card presentation comes from the theme (theme.cards), not hardcoded, so
		// the look is portable theme data. Headings mirror the PROMPT card.
		const cards = theme.cards;
		const glyph = cards.headingGlyph === "model" ? this.modelGlyph : cards.headingGlyph || undefined;
		const showHeadings = cards.showHeadings && !!glyph;
		const cardBg = bgAnsi ? (s: string) => `${bgAnsi}${s}\x1b[49m` : undefined;
		// Thoughts only carry the model tint when the theme opts in.
		const thinkingBg = cards.thinkingShaded ? cardBg : undefined;
		const bodyIndent = showHeadings ? cards.bodyIndent : cards.headingIndent;
		let responseHeadingRendered = false;
		let thoughtHeadingRendered = false;
		const addHeading = (card: Box, label: string) => {
			if (!showHeadings) return;
			card.addChild(new Text(accent(theme.bold(`${glyph} ${label}`)), cards.headingIndent, 0));
		};

		// Render content blocks in order.
		for (let i = 0; i < message.content.length; i++) {
			const content = message.content[i];

			// ── Text (response) block ─────────────────────────────────
			if (content.type === "text" && content.text.trim()) {
				// Wrap text in a shaded card with model-tinted background
				const textCard = new Box(cards.cardPadX, cards.cardPadY, cardBg);
				if (!responseHeadingRendered) {
					addHeading(textCard, cards.responseLabel);
					responseHeadingRendered = true;
				}
				textCard.addChild(new Markdown(content.text.trim(), bodyIndent, 0, this.markdownTheme));
				this.contentContainer.addChild(textCard);
			}

			// ── Thinking block ───────────────────────────────────────
			else if (content.type === "thinking" && content.thinking.trim()) {
				if (this.hideThinkingBlock) {
					const label = theme.italic(theme.fg("thinkingText", this.hiddenThinkingLabel));
					this.contentContainer.addChild(new Text(label, bodyIndent, 0));
				} else {
					// Thinking body: dim, italic; shaded only when theme.cards.thinkingShaded.
					const thinkingCard = new Box(cards.cardPadX, cards.cardPadY, thinkingBg);
					if (!thoughtHeadingRendered) {
						addHeading(thinkingCard, cards.thoughtLabel);
						thoughtHeadingRendered = true;
					}
					thinkingCard.addChild(
						new Markdown(content.thinking.trim(), bodyIndent, 0, this.markdownTheme, {
							color: (t: string) => theme.fg("thinkingText", t),
							italic: true,
						}),
					);
					this.contentContainer.addChild(thinkingCard);
				}
			}

			// Consecutive thinking/text blocks are each rendered in their own
			// shaded Box, whose top/bottom padding already separates them. An
			// extra Spacer here injected an *unshaded* blank line between two
			// shaded ones, making the gap look inconsistent — so we don't add one.
		}

		// ── Stop-reason status ───────────────────────────────────────
		const hasToolCalls = message.content.some((c) => c.type === "toolCall");
		this.hasToolCalls = hasToolCalls;
		if (!hasToolCalls) {
			if (message.stopReason === "aborted") {
				const msg =
					message.errorMessage && message.errorMessage !== "Request was aborted"
						? message.errorMessage
						: "Operation aborted";
				this.contentContainer.addChild(new Spacer(1));
				this.contentContainer.addChild(new Text(theme.fg("error", msg), 1, 0));
			} else if (message.stopReason === "error") {
				const errorMsg = message.errorMessage || "Unknown error";
				this.contentContainer.addChild(new Spacer(1));
				this.contentContainer.addChild(new Text(theme.fg("error", `Error: ${errorMsg}`), 1, 0));
			}
		}
	}
}
