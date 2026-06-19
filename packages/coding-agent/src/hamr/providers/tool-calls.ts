import type { ParsedModelOutput, ParseWarning } from "./types.ts";
import type { ParsedToolCall } from "./parsers/types.ts";
import { toolCallParserRegistry, ensureParsersRegistered } from "./parsers/index.ts";
import { sanitizeReasoningTags as sanitizeReasoning } from "./parsers/utils.ts";
import { repairJson } from "./repair/json-repair.ts";
import { sanitizeReasoning as repairSanitize } from "./repair/reasoning-sanitizer.ts";
import { repairXml } from "./repair/xml-repair.ts";

export { toolCallParserRegistry } from "./parsers/index.ts";

let parsersEnsured = false;
function ensureReg(): void {
	if (!parsersEnsured) {
		parsersEnsured = true;
		ensureParsersRegistered();
	}
}

/**
 * Parse raw model output into a typed ParsedModelOutput.
 */
export function parseModelOutput(content: string, parserId: string, reasoningContent?: string): ParsedModelOutput {
	ensureReg();
	const warnings: ParseWarning[] = [];
	const reasoning = reasoningContent?.trim() || undefined;
	let cleanedContent = content;

	// Extract reasoning from content if the provider embeds it inline
	// (Qwen models emit <think> blocks, some models leak thinking in various forms).
	// Use the dedicated reasoning sanitizer for provider-aware extraction.
	if (!reasoning) {
		const sanitizeResult = repairSanitize(content);
		if (sanitizeResult.removedReasoning) {
			warnings.push({ message: "Extracted reasoning tags from model output", source: "reasoning" });
		}
		cleanedContent = sanitizeResult.content;
	} else {
		// If reasoning was provided via API field, also strip any inline tags
		// from the content so they don't interfere with parsing.
		const stripped = sanitizeReasoning(cleanedContent);
		if (stripped !== cleanedContent) {
			warnings.push({ message: "Stripped inline reasoning tags from content", source: "reasoning" });
			cleanedContent = stripped;
		}
	}

	// Parse tool calls from the cleaned content, with repair fallback.
	const parserResult = toolCallParserRegistry.parse(parserId, cleanedContent);
	let parserOk = parserResult.ok;
	const toolCalls: ParsedToolCall[] = [];
	if (parserResult.ok) {
		toolCalls.push(...parserResult.calls);
	} else {
		// Repair cascade: try JSON repair first, then XML repair.
		// Track attempted (parserId, content) pairs to avoid redundant re-parsing
		// when repair functions produce identical or unchanged output (#16).
		const attemptedRepairs = new Set<string>();
		attemptedRepairs.add(`${parserId}::${cleanedContent}`); // initial parse already attempted

		const tryParseRepaired = (pid: string, text: string) => {
			const key = `${pid}::${text}`;
			if (attemptedRepairs.has(key)) return null;
			attemptedRepairs.add(key);
			return toolCallParserRegistry.parse(pid, text);
		};

		const jsonRepaired = repairJson(cleanedContent);
		if (jsonRepaired) {
			const result = tryParseRepaired(parserId, jsonRepaired.repaired);
			if (result?.ok && result.calls.length > 0) {
				parserOk = true;
				warnings.push({ message: "Recovered via JSON repair", source: "parser" });
				toolCalls.push(...result.calls);
			}
		}
		if (toolCalls.length === 0) {
			const xmlRepaired = repairXml(cleanedContent);
			if (xmlRepaired) {
				const result = tryParseRepaired(parserId, xmlRepaired.repaired);
				if (result?.ok && result.calls.length > 0) {
					parserOk = true;
					warnings.push({ message: "Recovered via XML repair", source: "parser" });
					toolCalls.push(...result.calls);
				} else if (parserId !== "qwen3_xml") {
					const qwenResult = tryParseRepaired("qwen3_xml", xmlRepaired.repaired);
					if (qwenResult?.ok && qwenResult.calls.length > 0) {
						parserOk = true;
						warnings.push({ message: "Recovered via XML repair + qwen3_xml parser", source: "parser" });
						toolCalls.push(...qwenResult.calls);
					}
				}
			}
		}
		if (toolCalls.length === 0) {
			warnings.push({ message: parserResult.error ?? "parser error", source: "parser" });
		}
	}

	// If no tool calls found in content, try parsing reasoning_content.
	// Some providers route tool-call XML to reasoning_content instead of content.
	if (toolCalls.length === 0 && reasoning) {
		let rcResult = toolCallParserRegistry.parse(parserId, reasoning);
		if (!rcResult.ok || rcResult.calls.length === 0) {
			const cleanedReasoning = sanitizeReasoning(reasoning);
			if (cleanedReasoning) {
				rcResult = toolCallParserRegistry.parse(parserId, cleanedReasoning);
			}
		}
		if ((!rcResult.ok || rcResult.calls.length === 0) && parserId !== "qwen3_xml") {
			const xmlResult = toolCallParserRegistry.parse("qwen3_xml", reasoning);
			if (xmlResult.ok && xmlResult.calls.length > 0) {
				rcResult = xmlResult;
			} else {
				const cleanedReasoning = sanitizeReasoning(reasoning);
				if (cleanedReasoning) {
					const xmlResult2 = toolCallParserRegistry.parse("qwen3_xml", cleanedReasoning);
					if (xmlResult2.ok && xmlResult2.calls.length > 0) {
						rcResult = xmlResult2;
					}
				}
			}
		}
		if (rcResult.ok && rcResult.calls.length > 0) {
			warnings.push({ message: "Extracted tool calls from reasoning_content", source: "parser" });
			toolCalls.push(...rcResult.calls);
		}
	}

	// Extract assistant-visible text (content without tool-call blocks)
	let assistantText = parserResult.content || cleanedContent;

	// Bug #114: When DeepSeek returns empty content but rich reasoning_content,
	// fall back to reasoning as the assistant-visible answer. Strip thinking/tool-call
	// tags from reasoning to produce clean prose.
	if (!assistantText && reasoning) {
		const sanitizedReasoning = sanitizeReasoning(reasoning);
		// Also strip tool-call markup that may have leaked into reasoning_content
		const visible = sanitizedReasoning
			.replace(/<tool_call>[\s\S]*?<\/tool_call>/gi, " ")
			.replace(/<\|tool_call\|>[\s\S]*/gi, "")
			.trim();
		if (visible) {
			assistantText = visible;
			warnings.push({ message: "Used reasoningContent as fallback for empty content (bug #114)", source: "reasoning" });
		}
	}

	return {
		assistantText,
		toolCalls,
		reasoning,
		warnings,
		parserOk,
	};
}
