import { createHash } from "node:crypto";
import type { AgentMessage } from "@hamr/agent";
import type { AssistantMessage, TextContent, ToolCall, ToolResultMessage } from "@hamr/ai";
import type { ExtensionContext, ExtensionFactory, SessionBeforeCompactEvent } from "../../core/extensions/types.ts";
import { registerHandoffTool } from "../handoff/HandoffManager.ts";
import { contentText } from "../helpers.ts";
import type { HolographicMemory } from "../memory/HolographicMemory.ts";
import {
	getFactStore,
	getMemory,
	registerMemoryTools,
	sanitizeMemoryTranscriptText,
	setCurrentTurnId,
	storeMessage,
} from "../memory.ts";
import { isCloudProvider, loadHamrStartupConfig } from "../startup-config.ts";

// ─── Auto-inject gate ────────────────────────────────────────────────────────

/**
 * Default token budget for auto-injected context (characters / 4 ≈ tokens).
 * Controlled via HAMR_MEMORY_AUTO_INJECT_TOKEN_BUDGET. Default 400 tokens.
 */
const AUTO_INJECT_TOKEN_BUDGET = (() => {
	const raw = process.env.HAMR_MEMORY_AUTO_INJECT_TOKEN_BUDGET;
	if (raw === undefined) return 400;
	const parsed = Number.parseInt(raw, 10);
	return Number.isNaN(parsed) || parsed < 0 ? 400 : parsed;
})();
const AUTO_INJECT_CHAR_BUDGET = AUTO_INJECT_TOKEN_BUDGET * 4;

function isTruthy(value: string | undefined): boolean {
	return value === "1" || value === "true";
}

/** Track sessions that have already received their one-shot context injection. */
const injectedSessions = new Set<string>();

function clearInjectedSession(sessionId: string): void {
	injectedSessions.delete(sessionId);
}

/** Track the last injected context hash per session for de-duplication. */
const contextHashState = new Map<string, string>();

function clearContextHashState(sessionId: string): void {
	contextHashState.delete(sessionId);
}

/**
 * De-duplicate auto-results against existing context messages.
 * Removes result lines whose core content already appears in any existing message.
 */
export function deduplicateResults(autoResults: string[], existingMessages: unknown[]): string[] {
	if (autoResults.length === 0 || existingMessages.length === 0) return autoResults;

	// Build a set of already-present trigrams from existing messages
	const seen = new Set<string>();
	for (const msg of existingMessages) {
		if (typeof msg === "object" && msg !== null && "content" in msg) {
			const content = (msg as { content: unknown }).content;
			if (typeof content === "string") {
				const words = content
					.toLowerCase()
					.split(/\s+/)
					.filter((w) => w.length > 1);
				for (let i = 0; i < words.length - 2; i++) {
					const phrase = words.slice(i, i + 3).join(" ");
					if (phrase.length >= 10) seen.add(phrase);
				}
			}
		}
	}

	return autoResults.filter((line) => {
		// Always keep index/header lines
		if (line.startsWith("// Search") || line.startsWith("[")) return true;

		// Extract the meaningful content after the metadata prefix (e.g. "//   turn 2 assistant:")
		const contentMatch = line.match(/^\/\/\s+turn\s+\d+\s+\S+:\s*(.+)/);
		const content = contentMatch ? contentMatch[1].toLowerCase() : line.toLowerCase();
		const words = content.split(/\s+/).filter((w) => w.length > 1);

		// Generate trigrams from the result content
		const resultTrigrams: string[] = [];
		for (let i = 0; i < words.length - 2; i++) {
			resultTrigrams.push(words.slice(i, i + 3).join(" "));
		}

		// If any meaningful trigram overlaps with existing context, this is a duplicate
		for (const trigram of resultTrigrams) {
			if (trigram.length >= 10 && seen.has(trigram)) return false;
		}
		return true;
	});
}

/**
 * Apply token budget cap to auto-results.
 * Truncates from the end, keeping the most relevant (first) results.
 * Preserves search header lines.
 */
export function applyTokenBudget(autoResults: string[], charBudget: number): string[] {
	if (charBudget <= 0) return autoResults;

	const truncated: string[] = [];
	let charsUsed = 0;

	for (const line of autoResults) {
		const lineChars = line.length + 1; // +1 for newline
		if (charsUsed + lineChars > charBudget) {
			// If we can't fit this whole line, truncate it if it's a result line
			if (!line.startsWith("// Search") && !line.startsWith("[")) {
				const remaining = charBudget - charsUsed;
				if (remaining > 20) {
					truncated.push(`${line.slice(0, remaining - 1)}…`);
				}
			}
			break;
		}
		truncated.push(line);
		charsUsed += lineChars;
	}

	return truncated;
}

function hashContext(autoResults: string[], index: string, survivalManifest?: string | null): string {
	return createHash("sha256")
		.update(autoResults.join("\n"))
		.update(index)
		.update(survivalManifest ?? "")
		.digest("hex")
		.slice(0, 16);
}

/**
 * Builds the user message injected into context from FTS5 auto-retrieval.
 *
 * A survival manifest (from a prior local-model compaction) is surfaced first
 * and prominently — it is the resumed instance's primary orientation, not a
 * generic search hit. Returns null only when there is nothing worth injecting.
 */
export function buildMemoryContextMessage(
	autoResults: string[],
	index: string,
	options: { survivalManifest?: string | null; timestamp?: number } = {},
): { role: "user"; content: string; timestamp: number } | null {
	const { survivalManifest, timestamp = Date.now() } = options;
	if (autoResults.length === 0 && !survivalManifest) return null;

	const sections: string[] = [];
	if (survivalManifest) {
		sections.push(
			`\nSURVIVAL MANIFEST (most recent local-model compaction — your primary orientation; use search_memory on the keys to recover full detail):\n${survivalManifest}`,
		);
	}
	if (autoResults.length > 0) {
		sections.push(`\nAuto-retrieved context from prior sessions:\n${autoResults.join("\n")}`);
	}
	sections.push(`\n${index}`);
	return { role: "user", content: sections.join("\n"), timestamp };
}

// ─── Local compaction tiers ─────────────────────────────────────────────────

export type LocalCompactionTier = "cloud" | "local-131k" | "local-64k" | "local-32k" | "local-16k";

export interface LocalCompactionPolicy {
	tier: LocalCompactionTier;
	contextWindow: number;
	keyLimit: number;
	searchTermLimit: number;
	resultsPerTerm: number;
	snippetChars: number;
	instructions: string;
}

export function selectCompactionPolicy(options: { cloud: boolean; contextWindow?: number }): LocalCompactionPolicy {
	const contextWindow = options.contextWindow && options.contextWindow > 0 ? options.contextWindow : 16_384;
	if (options.cloud) {
		return {
			tier: "cloud",
			contextWindow,
			keyLimit: 8,
			searchTermLimit: 5,
			resultsPerTerm: 3,
			snippetChars: 180,
			instructions: "Use pi's default LLM compaction; FTS5 stores a structured bookkeeping handoff.",
		};
	}
	if (contextWindow >= 96_000) {
		return {
			tier: "local-131k",
			contextWindow,
			keyLimit: 10,
			searchTermLimit: 6,
			resultsPerTerm: 3,
			snippetChars: 180,
			instructions:
				"Large local context: keep a compact structured handoff and recover deeper provenance from FTS5 only as needed.",
		};
	}
	if (contextWindow >= 49_152) {
		return {
			tier: "local-64k",
			contextWindow,
			keyLimit: 12,
			searchTermLimit: 5,
			resultsPerTerm: 2,
			snippetChars: 160,
			instructions: "Medium local context: prefer targeted FTS5 recovery over broad automatic replay.",
		};
	}
	if (contextWindow >= 24_576) {
		return {
			tier: "local-32k",
			contextWindow,
			keyLimit: 14,
			searchTermLimit: 4,
			resultsPerTerm: 2,
			snippetChars: 140,
			instructions:
				"Small local context: carry only decisions/status plus search keys; call search_memory for details.",
		};
	}
	return {
		tier: "local-16k",
		contextWindow,
		keyLimit: 16,
		searchTermLimit: 3,
		resultsPerTerm: 1,
		snippetChars: 120,
		instructions:
			"Tiny local context: do not replay history. Use the manifest as a map, then recover one key at a time with search_memory.",
	};
}

// ─── Survival manifest ─────────────────────────────────────────────────────

/**
 * The survival manifest is NOT a summary — it is a small map back into FTS5 for
 * a local/relay model that cannot afford an LLM compaction call near its limit.
 * Its job is to carry the few things a cold-resumed instance can't reconstruct
 * by searching: the verbatim task, ground-truth status, the planned next action,
 * and a handful of specific FTS5 keys that each recover something important.
 */
export interface SurvivalData {
	/** Tier policy selected for this compaction. */
	tier: LocalCompactionTier;
	/** Context window used to select the tier. */
	contextWindow: number;
	/** Verbatim goal — can't be searched for because the resumed instance doesn't know the words. */
	task: string;
	/** Ground truth right now: files modified, last command + result, branch. */
	status: string[];
	/** The next concrete action that was planned (lives only in the last assistant message). */
	next: string;
	/** 4-8 specific FTS5 search terms that each recover something important. */
	keys: string[];
	/** Files and identifiers that explain where the keys came from. */
	provenance: string[];
	/** Recovery guidance tuned for the selected tier. */
	instructions: string;
}

const INTENT_RE = /\b(i'?ll|i will|let me|let'?s|going to|next,? i|next step|the fix|plan to|need to|then i)\b/i;
const EDIT_TOOLS = new Set(["edit", "write", "str_replace", "str_replace_editor", "create_file", "apply_patch"]);
const PATH_ARG_KEYS = ["file_path", "path", "filePath", "filename", "file"];

function assistantText(message: AssistantMessage): string {
	return message.content
		.filter((part): part is TextContent => part.type === "text")
		.map((part) => part.text)
		.join("")
		.trim();
}

function toolCalls(message: AssistantMessage): ToolCall[] {
	return message.content.filter((part): part is ToolCall => part.type === "toolCall");
}

function argPath(args: Record<string, unknown> | undefined): string | undefined {
	if (!args) return undefined;
	for (const key of PATH_ARG_KEYS) {
		const value = args[key];
		if (typeof value === "string" && value.trim()) return value.trim();
	}
	return undefined;
}

function splitSentences(text: string): string[] {
	return text
		.split(/(?<=[.!?])\s+|\n+/)
		.map((s) => s.trim())
		.filter(Boolean);
}

function truncate(text: string, max: number): string {
	const clean = text.replace(/\s+/g, " ").trim();
	return clean.length > max ? `${clean.slice(0, max - 1)}…` : clean;
}

/** Parse a `git switch`/`git checkout [-b]` command into the target branch name. */
function parseBranch(command: string): string | undefined {
	const match = command.match(/git\s+(?:switch|checkout)\s+(?:-[bc]\s+)?([\w./-]+)/);
	const name = match?.[1];
	if (!name || name === "-" || name.startsWith("-")) return undefined;
	return name;
}

/** Extract the first specific error line from tool-result text. */
function firstErrorLine(text: string): string | undefined {
	for (const raw of text.split("\n")) {
		const line = raw.trim();
		if (line.length < 8 || line.length > 200) continue;
		if (/\b(error|failed|failure|exception|cannot|not found|undefined)\b/i.test(line)) {
			return truncate(line, 160);
		}
	}
	return undefined;
}

/** Backticked tokens are how assistants name the specific identifiers central to the work. */
function backtickedIdentifiers(text: string): string[] {
	const out: string[] = [];
	for (const match of text.matchAll(/`([^`\n]{2,60})`/g)) {
		const token = match[1].trim();
		// Keep specific identifiers/paths; drop prose-y multi-word backtick spans.
		if (token && !/\s/.test(token)) out.push(token);
	}
	return out;
}

export function extractSurvivalData(
	messages: AgentMessage[],
	policy: LocalCompactionPolicy = selectCompactionPolicy({ cloud: false, contextWindow: 16_384 }),
): SurvivalData {
	const assistants: AssistantMessage[] = [];
	const orderedCalls: ToolCall[] = [];
	const resultsByCallId = new Map<string, { isError: boolean; text: string }>();
	let firstUserText = "";

	for (const message of messages) {
		if (message.role === "user") {
			const text = contentText((message as { content: unknown }).content).trim();
			if (!firstUserText && text) firstUserText = text;
		} else if (message.role === "assistant") {
			const assistant = message as AssistantMessage;
			assistants.push(assistant);
			for (const call of toolCalls(assistant)) orderedCalls.push(call);
		} else if (message.role === "toolResult") {
			const result = message as ToolResultMessage;
			resultsByCallId.set(result.toolCallId, { isError: result.isError, text: contentText(result.content) });
		}
	}

	// Task: the verbatim goal. The user's first message is the truest source;
	// fall back to the earliest assistant text when no user message survived.
	const firstAssistantText = assistants.map(assistantText).find((t) => t.length > 0) ?? "";
	const taskSource = firstUserText || firstAssistantText;
	const task = taskSource ? truncate(splitSentences(taskSource).slice(0, 2).join(" "), 240) : "(task not recovered)";

	// Next: the planned action lives in the last assistant message(s).
	let next = "";
	for (let i = assistants.length - 1; i >= 0 && !next; i--) {
		const sentences = splitSentences(assistantText(assistants[i]));
		const intent = sentences.find((s) => INTENT_RE.test(s));
		if (intent) next = truncate(intent, 200);
	}
	if (!next) next = "(no explicit next action recorded)";

	// Status: ground truth from tool calls/results.
	const modified = new Set<string>();
	let lastBash: { command: string; result?: { isError: boolean; text: string } } | undefined;
	let branch: string | undefined;
	for (const call of orderedCalls) {
		if (EDIT_TOOLS.has(call.name)) {
			const path = argPath(call.arguments);
			if (path) modified.add(path);
		} else if (call.name === "bash" && typeof call.arguments?.command === "string") {
			const command = call.arguments.command as string;
			lastBash = { command, result: resultsByCallId.get(call.id) };
			const parsed = parseBranch(command);
			if (parsed) branch = parsed;
		}
	}

	const status: string[] = [];
	if (modified.size > 0) status.push(`modified: ${Array.from(modified).slice(0, 8).join(", ")}`);
	if (lastBash) {
		const outcome = lastBash.result ? (lastBash.result.isError ? "failed" : "succeeded") : "result unknown";
		status.push(`last command: \`${truncate(lastBash.command, 100)}\` (${outcome})`);
	}
	if (branch) status.push(`branch: ${branch}`);

	// Keys: specific terms that each recover something lossless from FTS5.
	const keys: string[] = [];
	const provenance: string[] = [];
	const seen = new Set<string>();
	const add = (candidate: string | undefined): void => {
		if (keys.length >= policy.keyLimit || !candidate) return;
		const value = candidate.trim();
		if (!value || value.length > 160 || seen.has(value.toLowerCase())) return;
		seen.add(value.toLowerCase());
		keys.push(value);
	};
	const addProvenance = (candidate: string | undefined): void => {
		if (!candidate || provenance.length >= 12) return;
		const value = candidate.trim();
		if (!value || provenance.some((p) => p.toLowerCase() === value.toLowerCase())) return;
		provenance.push(value);
	};

	// 1. Exact error strings being fought.
	for (const { isError, text } of resultsByCallId.values()) {
		if (keys.length >= policy.keyLimit) break;
		if (isError || /\b(error|failed|exception)\b/i.test(text)) add(firstErrorLine(text));
	}
	// 2. Specific file paths actively being worked in.
	for (const call of orderedCalls) {
		if (keys.length >= policy.keyLimit) break;
		const path = argPath(call.arguments);
		add(path);
		addProvenance(path);
	}
	// 3. Specific identifiers / decisions named in assistant text.
	for (const assistant of assistants) {
		if (keys.length >= policy.keyLimit) break;
		for (const id of backtickedIdentifiers(assistantText(assistant))) add(id);
	}

	for (const call of orderedCalls.slice(-8)) {
		addProvenance(`${call.name}${argPath(call.arguments) ? `:${argPath(call.arguments)}` : ""}`);
	}

	return {
		tier: policy.tier,
		contextWindow: policy.contextWindow,
		task,
		status,
		next,
		keys,
		provenance,
		instructions: policy.instructions,
	};
}

export function formatSurvivalManifest(data: SurvivalData): string {
	const lines = [
		"## Survival manifest (local-model compaction)",
		`Tier: ${data.tier} (${data.contextWindow.toLocaleString()} token window)`,
		`Task: ${data.task}`,
		`Recovery: ${data.instructions}`,
	];
	if (data.status.length > 0) {
		lines.push("Status:", ...data.status.map((s) => `- ${s}`));
	} else {
		lines.push("Status: (no concrete state recorded)");
	}
	lines.push(`Next: ${data.next}`);
	if (data.keys.length > 0) {
		lines.push("Search keys (use search_memory to recover full detail from FTS5):");
		for (const key of data.keys) lines.push(`- ${key}`);
	}
	if (data.provenance.length > 0) {
		lines.push("Provenance anchors:");
		for (const item of data.provenance) lines.push(`- ${item}`);
	}
	return lines.join("\n");
}

/** Build the survival manifest string for a set of messages about to be discarded. */
export function buildSurvivalManifest(messages: AgentMessage[], policy?: LocalCompactionPolicy): string {
	return formatSurvivalManifest(extractSurvivalData(messages, policy));
}

/**
 * Cloud bookkeeping handoff: a lightweight manifest stored in FTS5 so the
 * resumed (LLM-summarized) session still has structured search hooks. Cloud
 * models keep pi's default compaction, so this never overrides the summary.
 */
function storeCompactionHandoff(
	memory: HolographicMemory,
	ctx: ExtensionContext,
	event: SessionBeforeCompactEvent,
): void {
	const manifest = memory.handoff();
	memory.store({
		sessionId: ctx.sessionManager.getSessionId(),
		turnId: 0,
		role: "tool",
		toolName: "compaction_handoff",
		content: JSON.stringify(
			{
				task: "Compaction handoff from pi auto-compaction",
				manifest,
				branchEntries: event.branchEntries?.length ?? 0,
			},
			null,
			2,
		),
		domainTags: ["hamr", "compaction"],
	});
}

/**
 * Memory extension: FTS5 memory tools (search/save/handoff), message storage,
 * a two-path compaction strategy, and the turn counter. Orthogonal to session
 * topology — purely about persistence.
 */
export const hamrMemoryExtension: ExtensionFactory = async (pi) => {
	registerMemoryTools(pi);
	registerHandoffTool(pi);

	// Cloud vs local is decided from the startup config (loaded once — compaction
	// is rare and the config is just a TOML read).
	const config = loadHamrStartupConfig(process.cwd());

	// Store every completed message into FTS5 memory.
	pi.on("message_end", (event, ctx) => {
		storeMessage(ctx, event.message);
	});

	// Two-path compaction. Cloud models have headroom for pi's LLM summary, so we
	// only record a bookkeeping manifest and let pi proceed. Local/relay models
	// do NOT have headroom near the limit — instead of an LLM call we emit a small
	// survival manifest that maps back into FTS5, and hand it to pi as the
	// compaction override so it becomes the resumed session's carried context.
	pi.on("session_before_compact", (event, ctx) => {
		const memory = getMemory(ctx);
		if (!memory) return; // FTS5 unavailable → silent no-op, pi handles it.

		const provider = ctx.model?.provider;
		// Missing model → treat as cloud (safe default: pi handles compaction).
		const cloud = !provider || isCloudProvider(config, provider);
		const policy = selectCompactionPolicy({ cloud, contextWindow: ctx.model?.contextWindow });
		if (cloud) {
			storeCompactionHandoff(memory, ctx, event);
			return; // undefined → pi runs its default LLM compaction.
		}

		const manifest = buildSurvivalManifest(event.preparation.messagesToSummarize, policy);
		memory.store({
			sessionId: ctx.sessionManager.getSessionId(),
			turnId: 0,
			role: "tool",
			toolName: "survival_manifest",
			content: manifest,
			domainTags: ["hamr", "survival", "compaction"],
		});
		return {
			compaction: {
				summary: manifest,
				firstKeptEntryId: event.preparation.firstKeptEntryId,
				tokensBefore: event.preparation.tokensBefore,
			},
		};
	});

	// Context injection: auto-search memory and append retrieved context for
	// resumed/handoff sessions. Gated behind HAMR_MEMORY_AUTO_INJECT.
	//
	// When disabled (default), the model must explicitly call search_memory.
	// When enabled, injected ONCE per session on the first turn that has prior
	// session entries — not every turn. Includes a token budget cap and
	// de-duplication against existing context.
	//
	// Memory context is APPENDED (not prepended) to preserve Anthropic's
	// longest-prefix prompt caching.
	pi.on("context", (event, ctx) => {
		// ── Opt-in gate: skip auto-injection unless HAMR_MEMORY_AUTO_INJECT is truthy ──
		if (!isTruthy(process.env.HAMR_MEMORY_AUTO_INJECT)) return;

		const memory = getMemory(ctx);
		if (!memory) return;

		const sessionId = ctx.sessionManager.getSessionId();

		// ── Only inject on resumed/handoff sessions (sessions with prior entries) ──
		if (!memory.hasSessionEntries(sessionId)) return;

		// ── One-shot: inject only once per session, never on every turn ──
		if (injectedSessions.has(sessionId)) return;

		let index = memory.buildMemoryIndex();

		// Append fact store status to the memory index
		const factStore = getFactStore(ctx);
		if (factStore?.isAvailable) {
			const fc = factStore.getFactCount();
			const fsLine =
				fc > 0
					? `\n[FactStore: ${fc} durable facts with entity resolution & trust scoring. Use fact_store to query, fact_feedback to rate.]`
					: `\n[FactStore: active, empty. Use fact_store(action='add') to persist cross-session knowledge.]`;
			index = index ? `${index}${fsLine}` : fsLine;
		}
		if (!index) return;

		const survival = memory.getLatestByDomainTag("survival", sessionId);
		const survivalManifest = survival?.content ?? null;
		const provider = ctx.model?.provider;
		const cloud = !provider || isCloudProvider(config, provider);

		// Cloud providers: skip auto-injection entirely unless a survival
		// manifest from a prior local compaction exists. Cloud models rely on
		// proper LLM compaction, not FTS5 context injection.
		if (cloud && !survivalManifest) return;

		const policy = selectCompactionPolicy({ cloud, contextWindow: ctx.model?.contextWindow });

		const terms = memory.getSuggestedSearchTerms();
		const autoResults: string[] = [];
		for (const term of terms.slice(0, policy.searchTermLimit)) {
			const results = memory.searchWithSnippets(term, policy.resultsPerTerm);
			if (results.length > 0) {
				autoResults.push(
					`// Search "${term}": ${results.length} results`,
					...results.map(
						(r) =>
							`//   turn ${r.turnId} ${r.role}${r.toolName ? `/${r.toolName}` : ""}: ${sanitizeMemoryTranscriptText(
								r.snippet || r.content.slice(0, policy.snippetChars),
							)}`,
					),
				);
			}
		}

		// ── De-duplicate against existing context messages ──
		const deduped = deduplicateResults(autoResults, event.messages as unknown[]);

		// ── Apply token budget cap ──
		const budgeted = applyTokenBudget(deduped, AUTO_INJECT_CHAR_BUDGET);

		// ── De-duplicate by content hash (skip if same content was already injected) ──
		const contextHash = hashContext(budgeted, index, survivalManifest);
		const prior = contextHashState.get(sessionId);
		if (prior === contextHash) return;
		contextHashState.set(sessionId, contextHash);

		const message = buildMemoryContextMessage(budgeted, index, { survivalManifest });
		if (!message) return;

		// Mark session as injected — one-shot only
		injectedSessions.add(sessionId);

		// APPEND memory context after existing messages to preserve prefix cache.
		return { messages: [...event.messages, message] };
	});

	// Advance the memory turn counter at the end of each turn.
	pi.on("turn_end", (event) => {
		setCurrentTurnId(event.turnIndex + 1);
	});

	// Clean up context injection state when session shuts down.
	pi.on("session_shutdown", (_, ctx) => {
		const sid = ctx.sessionManager.getSessionId();
		clearInjectedSession(sid);
		clearContextHashState(sid);
	});
};
