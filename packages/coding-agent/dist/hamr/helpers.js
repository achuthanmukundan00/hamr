export function modelKey(provider, model) {
    return `${provider}/${model}`;
}
export function contentText(content) {
    if (typeof content === "string")
        return content;
    if (!Array.isArray(content))
        return "";
    return content
        .filter((part) => typeof part === "object" && part !== null && "type" in part && part.type === "text")
        .map((part) => part.text)
        .join("");
}
export function getAssistantText(message) {
    return message.content
        .filter((part) => part.type === "text")
        .map((part) => part.text)
        .join("");
}
export function getThinkingText(message) {
    const thinking = message.content
        .filter((part) => part.type === "thinking")
        .map((part) => part.thinking)
        .join("\n")
        .trim();
    return thinking || undefined;
}
export function hasToolCalls(message) {
    return message.content.some((part) => part.type === "toolCall");
}
export function fileHints(text) {
    const matches = text.match(/(?:^|\s)([./~]?[A-Za-z0-9._@/-]+\.[A-Za-z0-9]{1,8})(?=\s|$|[:),])/g) ?? [];
    return [...new Set(matches.map((match) => match.trim()).filter((match) => match.length < 240))].slice(0, 12);
}
//# sourceMappingURL=helpers.js.map