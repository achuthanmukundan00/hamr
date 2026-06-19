export function stripFtsMarks(s: string): string {
	return s.replace(/<\/?mark>/g, "");
}
