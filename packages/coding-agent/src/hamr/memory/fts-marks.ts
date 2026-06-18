/** Strip FTS5 snippet highlight tags from text returned to the model. */
export function stripFtsMarks(s: string): string {
	return s.replace(/<\/?mark>/g, "");
}

/** Render FTS5 snippet highlight tags using a custom highlight function for TUI display. */
export function renderFtsMarks(s: string, highlightFn: (s: string) => string): string {
	return s.replace(/<mark>(.*?)<\/mark>/gs, (_, text) => highlightFn(text));
}
