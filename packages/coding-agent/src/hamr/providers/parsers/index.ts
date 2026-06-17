/**
 * Hamr native tool-call parsers — index and registration.
 *
 * Registers all vLLM-equivalent tool-call parsers into the singleton registry.
 * Import this module once at startup to enable all parsers.
 *
 * Parser IDs match vLLM's --tool-call-parser values where practical.
 * Aliases are registered only for backward compatibility (e.g., 'qwen3_coder' → 'qwen3_xml').
 */

import { createDeepseekV3Parser, createDeepseekV31Parser } from "./deepseek.ts";
import { createGenericParser } from "./generic.ts";
import { createGlm45Parser, createGlm47Parser, createStep3Parser, createStep3p5Parser } from "./glm-step.ts";
import { createHermesParser } from "./hermes.ts";
import {
	createFunctionGemmaParser,
	createGigachat3Parser,
	createGranite4Parser,
	createGranite20bFcParser,
	createGraniteParser,
	createHunyuanA13bParser,
	createInternlmParser,
	createJambaParser,
	createKimiK2Parser,
	createLongcatParser,
	createMinimaxParser,
	createOlmo3Parser,
	createOpenaiPassthroughParser,
} from "./json-in-tags.ts";
import { createLlama3JsonParser } from "./llama3-json.ts";
import { createMistralParser } from "./mistral.ts";
import { createLlama4PythonicParser, createPythonicParser } from "./pythonic.ts";
import { createQwen3XmlParser } from "./qwen3-xml.ts";
import { toolCallParserRegistry } from "./registry.ts";
import { createXlamParser } from "./xlam.ts";

// ─── Registration ─────────────────────────────────────────

let registered = false;

export function ensureParsersRegistered(): void {
	if (registered) return;
	registered = true;

	// XML/tag-based parsers (highest priority for local models)
	toolCallParserRegistry.register("qwen3_xml", createQwen3XmlParser);
	toolCallParserRegistry.register("qwen3_coder", createQwen3XmlParser); // backward compat alias
	toolCallParserRegistry.register("hermes", createHermesParser);
	toolCallParserRegistry.register("step3", createStep3Parser);
	toolCallParserRegistry.register("step3p5", createStep3p5Parser);
	toolCallParserRegistry.register("functiongemma", createFunctionGemmaParser);
	toolCallParserRegistry.register("gemma_native", createGenericParser);
	toolCallParserRegistry.register("olmo3", createOlmo3Parser);
	toolCallParserRegistry.register("glm45", createGlm45Parser);
	toolCallParserRegistry.register("glm47", createGlm47Parser);
	toolCallParserRegistry.register("gigachat3", createGigachat3Parser);

	// JSON-based parsers
	toolCallParserRegistry.register("llama3_json", createLlama3JsonParser);
	toolCallParserRegistry.register("mistral", createMistralParser);
	toolCallParserRegistry.register("xlam", createXlamParser);
	toolCallParserRegistry.register("granite", createGraniteParser);
	toolCallParserRegistry.register("granite4", createGranite4Parser);
	toolCallParserRegistry.register("granite-20b-fc", createGranite20bFcParser);
	toolCallParserRegistry.register("internlm", createInternlmParser);
	toolCallParserRegistry.register("jamba", createJambaParser);
	toolCallParserRegistry.register("minimax", createMinimaxParser);
	toolCallParserRegistry.register("kimi_k2", createKimiK2Parser);
	toolCallParserRegistry.register("hunyuan_a13b", createHunyuanA13bParser);
	toolCallParserRegistry.register("longcat", createLongcatParser);
	toolCallParserRegistry.register("openai", createOpenaiPassthroughParser);

	// Pythonic parsers
	toolCallParserRegistry.register("pythonic", createPythonicParser);
	toolCallParserRegistry.register("llama4_pythonic", createLlama4PythonicParser);

	// DeepSeek parsers
	toolCallParserRegistry.register("deepseek_v3", createDeepseekV3Parser);
	toolCallParserRegistry.register("deepseek_v31", createDeepseekV31Parser);

	// Generic fallback
	toolCallParserRegistry.register("generic", createGenericParser);
}

// ─── Re-exports ───────────────────────────────────────────

export { getToolCallParserRegistry, toolCallParserRegistry } from "./registry.ts";
export type {
	ParsedToolCall,
	ToolCallParseResult,
	ToolCallParser,
	ToolCallParserFactory,
	ToolCallParserRegistry,
} from "./types.ts";
export { detectParserId } from "./types.ts";

export {
	coerceValue,
	extractDelimitedBlocks,
	extractNonToolContent,
	fastJsonParse,
	generateCallId,
	makeCall,
	parsePythonicArgs,
	resetCallIdCounter,
	safeJsonParse,
	sanitizeReasoningTags,
} from "./utils.ts";
