import { Box, Container, Markdown, type MarkdownTheme, Text } from "@hamr/tui";
import { getMarkdownTheme, theme } from "../theme/theme.ts";

const OSC133_ZONE_START = "\x1b]133;A\x07";
const OSC133_ZONE_END = "\x1b]133;B\x07";
const OSC133_ZONE_FINAL = "\x1b]133;C\x07";

/**
 * Convert a hex color string (e.g. "#875fff") to an ANSI foreground color escape.
 */
function hexToAnsiFg(hex: string): string {
	const r = parseInt(hex.slice(1, 3), 16);
	const g = parseInt(hex.slice(3, 5), 16);
	const b = parseInt(hex.slice(5, 7), 16);
	return `\x1b[38;2;${r};${g};${b}m`;
}

/**
 * Component that renders a user message with a branded PROMPT card header.
 *
 * Every user message was sent to a specific model — the card heading always
 * shows that model's glyph + "PROMPT" so you can see which model you prompted
 * even after mid-session model switches. The heading color reflects the model's
 * brand accent when modelAdaptive is on, or the theme accent when off.
 */
export class UserMessageComponent extends Container {
	private contentBox: Box;

	constructor(
		text: string,
		markdownTheme: MarkdownTheme = getMarkdownTheme(),
		modelAccent?: string,
		modelGlyph?: string,
	) {
		super();

		// Card presentation comes from the theme (theme.cards) rather than being
		// hardcoded, so the "hamr look" is portable theme data.
		const cards = theme.cards;
		const glyph = cards.headingGlyph === "model" ? modelGlyph : cards.headingGlyph || undefined;
		const showHeading = cards.showHeadings && !!glyph;

		// Keep model color as an accent only. Using it as the card background
		// makes orange/red models dominate the entire prompt block.
		const promptBgFn = (content: string) => theme.bg("userMessageBg", content);
		this.contentBox = new Box(cards.cardPadX, cards.cardPadY, promptBgFn);

		// Show the glyph + label heading when configured. Uses model brand color
		// when modelAdaptive, theme accent otherwise.
		if (showHeading) {
			const headingColor =
				modelAccent && theme.modelAdaptive
					? (s: string) => `${hexToAnsiFg(modelAccent)}${s}\x1b[39m`
					: (s: string) => theme.fg("accent", s);
			this.contentBox.addChild(
				new Text(theme.bold(headingColor(`${glyph} ${cards.promptLabel}`)), cards.headingIndent, 0),
			);
		}

		// Indent the body so it nests under the label (past the glyph); without a
		// heading, keep the body at the base heading indent.
		const bodyIndent = showHeading ? cards.bodyIndent : cards.headingIndent;
		this.contentBox.addChild(
			new Markdown(
				text,
				bodyIndent,
				0,
				markdownTheme,
				{
					color: (content: string) => theme.fg("userMessageText", content),
				},
				{ preserveOrderedListMarkers: true },
			),
		);
		this.addChild(this.contentBox);
	}

	override render(width: number): string[] {
		const lines = super.render(width);
		if (lines.length === 0) {
			return lines;
		}

		lines[0] = OSC133_ZONE_START + lines[0];
		lines[lines.length - 1] = OSC133_ZONE_END + OSC133_ZONE_FINAL + lines[lines.length - 1];
		return lines;
	}
}
