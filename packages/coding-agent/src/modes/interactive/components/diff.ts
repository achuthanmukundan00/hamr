import { type Component, truncateToWidth, visibleWidth } from "@hamr/tui";
import { getLanguageFromPath, highlightCode, type ThemeBg, theme } from "../theme/theme.ts";

/**
 * A single logical diff row, normalized from either the internal edit-diff
 * format (produced by `generateDiffString`) or a raw git unified diff.
 */
interface DiffRow {
	kind: "added" | "removed" | "context" | "meta";
	/** Display line number (already stringified), or "" for meta/separator rows. */
	lineNum: string;
	content: string;
	/** Syntax-highlighting language for this row's content (per-file in multi-file diffs). */
	lang?: string;
}

/** Replace tabs with spaces for consistent column rendering. */
function replaceTabs(text: string): string {
	return text.replace(/\t/g, "   ");
}

/**
 * Parse the internal edit-diff format emitted by `generateDiffString`.
 * Lines look like: "+123 content", "-123 content", " 123 content", "     ...".
 */
function parseGeneratedDiff(diffText: string): DiffRow[] {
	const rows: DiffRow[] = [];
	for (const line of diffText.split("\n")) {
		const match = line.match(/^([+-\s])(\s*\d*)\s(.*)$/);
		if (!match) {
			rows.push({ kind: "meta", lineNum: "", content: line });
			continue;
		}
		const [, prefix, lineNum, content] = match;
		const trimmedNum = lineNum.trim();
		if (content.trim() === "..." && trimmedNum === "") {
			rows.push({ kind: "meta", lineNum: "", content: "⋯" });
			continue;
		}
		const kind = prefix === "+" ? "added" : prefix === "-" ? "removed" : "context";
		rows.push({ kind, lineNum: trimmedNum, content });
	}
	return rows;
}

/** Detect whether a blob of text is a raw git/unified diff. */
export function looksLikeUnifiedDiff(text: string): boolean {
	return /^@@ -\d/m.test(text) || /^diff --git /m.test(text);
}

/**
 * Parse a raw git unified diff into normalized rows, tracking line numbers from
 * the hunk headers. File headers (`diff --git`, `index`, `+++`, `---`) are
 * dropped; hunk headers become subtle separator rows. The syntax-highlighting
 * language is tracked per file, so a multi-file diff highlights each file's
 * lines with the right grammar instead of mislabeling (e.g. a shell comment
 * highlighted as TypeScript).
 */
function parseUnifiedDiff(diffText: string): DiffRow[] {
	const rows: DiffRow[] = [];
	let oldLine = 0;
	let newLine = 0;
	let lang: string | undefined;

	for (const line of diffText.split("\n")) {
		// Track the current file (and its language) as we cross file boundaries.
		const fileHeader = line.match(/^(?:diff --git a\/.+ b\/|\+\+\+ b\/)(.+)$/);
		if (fileHeader) {
			lang = getLanguageFromPath(fileHeader[1]);
			continue;
		}
		const hunk = line.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/);
		if (hunk) {
			oldLine = Number(hunk[1]);
			newLine = Number(hunk[2]);
			if (rows.length > 0) rows.push({ kind: "meta", lineNum: "", content: "⋯" });
			continue;
		}
		// Skip remaining file-level headers — they add noise without aiding review.
		if (/^(index |--- |new file mode |deleted file mode |similarity |rename |old mode |new mode )/.test(line)) {
			continue;
		}
		if (line.startsWith("+")) {
			rows.push({ kind: "added", lineNum: String(newLine++), content: line.slice(1), lang });
		} else if (line.startsWith("-")) {
			rows.push({ kind: "removed", lineNum: String(oldLine++), content: line.slice(1), lang });
		} else if (line.startsWith("\\")) {
			// "\ No newline at end of file" — ignore.
		} else {
			// Context line (leading space, or blank line inside a hunk).
			rows.push({
				kind: "context",
				lineNum: String(newLine),
				content: line.startsWith(" ") ? line.slice(1) : line,
				lang,
			});
			oldLine++;
			newLine++;
		}
	}
	return rows;
}

export interface RenderDiffOptions {
	/** File path used to choose a syntax-highlighting language. */
	filePath?: string;
	/** Treat input as a raw git/unified diff rather than the internal format. */
	unified?: boolean;
	/**
	 * Background the diff is painted onto. When the diff sits inside a shaded
	 * surface (e.g. a tool card), pass that surface's background so the band
	 * lines restore it after the colored band instead of resetting to the
	 * terminal default — otherwise the surrounding padding shows a mismatched
	 * strip on either side of the band.
	 */
	surroundBg?: ThemeBg;
}

/**
 * Width-aware diff component matching the "Claude Code" presentation:
 * code is always syntax-highlighted on a neutral base, and additions/removals
 * are conveyed by a full-width background band (green/red) rather than by
 * recoloring the code itself.
 */
class DiffComponent implements Component {
	private rows: DiffRow[];
	private lang?: string;
	private numWidth: number;
	private surroundBg?: ThemeBg;
	private cacheWidth?: number;
	private cacheLines?: string[];

	constructor(diffText: string, options: RenderDiffOptions = {}) {
		// Edit diffs are single-file, so the language comes from the supplied path.
		// Git diffs carry a per-row language resolved from each file header.
		this.lang = options.filePath ? getLanguageFromPath(options.filePath) : undefined;
		this.surroundBg = options.surroundBg;
		this.rows = options.unified ? parseUnifiedDiff(diffText) : parseGeneratedDiff(diffText);
		const maxNum = this.rows.reduce((w, r) => Math.max(w, r.lineNum.length), 1);
		this.numWidth = maxNum;
	}

	invalidate(): void {
		this.cacheWidth = undefined;
		this.cacheLines = undefined;
	}

	private highlight(content: string, lang: string | undefined): string {
		const text = replaceTabs(content);
		if (!text) return "";
		if (!lang) return text; // neutral base when language is unknown
		return highlightCode(text, lang)[0] ?? text;
	}

	private renderRow(row: DiffRow, width: number): string {
		if (row.kind === "meta") {
			const text = theme.fg("toolDiffContext", `${" ".repeat(this.numWidth + 2)}${row.content}`);
			return truncateToWidth(text, width);
		}

		const sign = row.kind === "added" ? "+" : row.kind === "removed" ? "-" : " ";
		const signColor =
			row.kind === "added" ? "toolDiffAdded" : row.kind === "removed" ? "toolDiffRemoved" : "toolDiffContext";
		const gutter = `${theme.fg(signColor, sign)}${theme.fg("toolDiffContext", row.lineNum.padStart(this.numWidth, " "))} `;

		const line = truncateToWidth(gutter + this.highlight(row.content, row.lang ?? this.lang), width);

		if (row.kind === "added") return this.band(line, width, "toolDiffAddedBg");
		if (row.kind === "removed") return this.band(line, width, "toolDiffRemovedBg");
		return line;
	}

	/**
	 * Paint a full-width background band behind a line. The band is closed by
	 * restoring the surrounding background (when known) rather than resetting to
	 * the terminal default, so any padding a parent box adds on either side
	 * stays the same color as the rest of the card.
	 */
	private band(content: string, width: number, bgToken: ThemeBg): string {
		const pad = " ".repeat(Math.max(0, width - visibleWidth(content)));
		const close = this.surroundBg ? theme.getBgAnsi(this.surroundBg) : "\x1b[49m";
		return theme.getBgAnsi(bgToken) + content + pad + close;
	}

	render(width: number): string[] {
		if (this.cacheLines && this.cacheWidth === width) return this.cacheLines;
		const lines = this.rows.map((row) => this.renderRow(row, width));
		this.cacheWidth = width;
		this.cacheLines = lines;
		return lines;
	}
}

/**
 * Create a width-aware diff component. Use this everywhere a diff is shown
 * (file edits and git diffs) so the presentation stays consistent.
 */
export function createDiffComponent(diffText: string, options: RenderDiffOptions = {}): Component {
	return new DiffComponent(diffText, options);
}
