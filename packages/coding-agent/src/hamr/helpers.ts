import type { AssistantMessage, TextContent } from "@hamr/ai";

export function modelKey(provider: string, model: string): string {
	return `${provider}/${model}`;
}

export function contentText(content: unknown): string {
	if (typeof content === "string") return content;
	if (!Array.isArray(content)) return "";
	return content
		.filter(
			(part): part is TextContent =>
				typeof part === "object" && part !== null && "type" in part && part.type === "text",
		)
		.map((part) => part.text)
		.join("");
}

export function getAssistantText(message: AssistantMessage): string {
	return message.content
		.filter((part): part is TextContent => part.type === "text")
		.map((part) => part.text)
		.join("");
}

export function getThinkingText(message: AssistantMessage): string | undefined {
	const thinking = message.content
		.filter((part) => part.type === "thinking")
		.map((part) => part.thinking)
		.join("\n")
		.trim();
	return thinking || undefined;
}

export function hasToolCalls(message: AssistantMessage): boolean {
	return message.content.some((part) => part.type === "toolCall");
}

export function fileHints(text: string): string[] {
	const matches = text.match(/(?:^|\s)([./~]?[A-Za-z0-9._@/-]+\.[A-Za-z0-9]{1,8})(?=\s|$|[:),])/g) ?? [];
	return [...new Set(matches.map((match) => match.trim()).filter((match) => match.length < 240))].slice(0, 12);
}
