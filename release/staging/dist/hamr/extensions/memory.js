import { createHash } from "node:crypto";
import { registerHandoffTool } from "../handoff/HandoffManager.js";
import { contentText } from "../helpers.js";
import { getFactStore, getMemory, registerMemoryTools, sanitizeMemoryTranscriptText, setCurrentTurnId, storeMessage, } from "../memory.js";
import { isCloudProvider, loadHamrStartupConfig } from "../startup-config.js";
// ─── Auto-inject gate ────────────────────────────────────────────────────────
/**
 * Default token budget for auto-injected context (characters / 4 ≈ tokens).
 * Controlled via HAMR_MEMORY_AUTO_INJECT_TOKEN_BUDGET. Default 400 tokens.
 */
const AUTO_INJECT_TOKEN_BUDGET = (() => {
    const raw = process.env.HAMR_MEMORY_AUTO_INJECT_TOKEN_BUDGET;
    if (raw === undefined)
        return 400;
    const parsed = Number.parseInt(raw, 10);
    return Number.isNaN(parsed) || parsed < 0 ? 400 : parsed;
})();
const AUTO_INJECT_CHAR_BUDGET = AUTO_INJECT_TOKEN_BUDGET * 4;
/**
 * Default budget for cue-triggered durable memory prefetch. This is separate
 * from full auto-injection: it only fires when the user asks to remember, pick
 * up context, or sends a likely continuation fragment ("the genre is...").
 */
const MEMORY_PREFETCH_TOKEN_BUDGET = (() => {
    const raw = process.env.HAMR_MEMORY_PREFETCH_TOKEN_BUDGET;
    if (raw === undefined)
        return 500;
    const parsed = Number.parseInt(raw, 10);
    return Number.isNaN(parsed) || parsed < 0 ? 500 : parsed;
})();
const MEMORY_PREFETCH_CHAR_BUDGET = MEMORY_PREFETCH_TOKEN_BUDGET * 4;
function isTruthy(value) {
    return value === "1" || value === "true";
}
function isFalsey(value) {
    return value === "0" || value === "false";
}
/** Track sessions that have already received their one-shot context injection. */
const injectedSessions = new Set();
function clearInjectedSession(sessionId) {
    injectedSessions.delete(sessionId);
}
/** Track the last injected context hash per session for de-duplication. */
const contextHashState = new Map();
function clearContextHashState(sessionId) {
    contextHashState.delete(sessionId);
}
/**
 * De-duplicate auto-results against existing context messages.
 * Removes result lines whose core content already appears in any existing message.
 */
export function deduplicateResults(autoResults, existingMessages) {
    if (autoResults.length === 0 || existingMessages.length === 0)
        return autoResults;
    // Build a set of already-present trigrams from existing messages
    const seen = new Set();
    for (const msg of existingMessages) {
        if (typeof msg === "object" && msg !== null && "content" in msg) {
            const content = contentText(msg.content);
            if (content) {
                const words = content
                    .toLowerCase()
                    .split(/\s+/)
                    .filter((w) => w.length > 1);
                for (let i = 0; i < words.length - 2; i++) {
                    const phrase = words.slice(i, i + 3).join(" ");
                    if (phrase.length >= 10)
                        seen.add(phrase);
                }
            }
        }
    }
    return autoResults.filter((line) => {
        // Always keep index/header lines
        if (line.startsWith("// Search") || line.startsWith("["))
            return true;
        // Extract the meaningful content after the metadata prefix (e.g. "//   turn 2 assistant:")
        const contentMatch = line.match(/^\/\/\s+turn\s+\d+\s+\S+:\s*(.+)/);
        const content = contentMatch ? contentMatch[1].toLowerCase() : line.toLowerCase();
        const words = content.split(/\s+/).filter((w) => w.length > 1);
        // Generate trigrams from the result content
        const resultTrigrams = [];
        for (let i = 0; i < words.length - 2; i++) {
            resultTrigrams.push(words.slice(i, i + 3).join(" "));
        }
        // If any meaningful trigram overlaps with existing context, this is a duplicate
        for (const trigram of resultTrigrams) {
            if (trigram.length >= 10 && seen.has(trigram))
                return false;
        }
        return true;
    });
}
/**
 * Apply token budget cap to auto-results.
 * Truncates from the end, keeping the most relevant (first) results.
 * Preserves search header lines.
 */
export function applyTokenBudget(autoResults, charBudget) {
    if (charBudget <= 0)
        return autoResults;
    const truncated = [];
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
function hashContext(autoResults, index, survivalManifest) {
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
export function buildMemoryContextMessage(autoResults, index, options = {}) {
    const { survivalManifest, timestamp = Date.now() } = options;
    if (autoResults.length === 0 && !survivalManifest)
        return null;
    const sections = [];
    if (survivalManifest) {
        sections.push(`\nSURVIVAL MANIFEST (most recent local-model compaction — your primary orientation; use search_memory on the keys to recover full detail):\n${survivalManifest}`);
    }
    if (autoResults.length > 0) {
        sections.push(`\nAuto-retrieved context from prior sessions:\n${autoResults.join("\n")}`);
    }
    sections.push(`\n${index}`);
    return { role: "user", content: sections.join("\n"), timestamp };
}
const EXPLICIT_RECALL_RE = /\b(?:remember|recall|last time|earlier|previous(?:ly)?|prior conversation|we talked|we were talking|where (?:we|it) left off|pick up|continue(?: from)?|that .{0,40}thing|the .{0,40}thing)\b/i;
const CONTINUATION_FRAGMENT_RE = /^(?:(?:the\s+(?:genre|vibe|artist|track|song|album|project|thing|one|issue|bug|error|problem|plan|approach|fix|branch|file|context|repo)\s+(?:is|was|are|were|=|should|needs?|has|uses?))|(?:(?:it|it's|its|that|this)\b)|(?:(?:also|btw)\b))/i;
const MUSIC_CONTEXT_RE = /\b(?:music|electronic|artist|genre|track|song|album|club|deconstructed|industrial|sound|vibe)\b/i;
const PROJECT_CONTEXT_RE = /\b(?:project|thing|context|conversation|remember|recall|last time|earlier|previous|pick up|continue)\b/i;
const PREFETCH_STOP_WORDS = new Set([
    "the",
    "a",
    "an",
    "and",
    "or",
    "but",
    "is",
    "are",
    "was",
    "were",
    "it",
    "its",
    "it's",
    "this",
    "that",
    "thing",
    "one",
    "about",
    "with",
    "from",
    "for",
    "to",
    "of",
    "in",
    "on",
    "can",
    "you",
    "we",
    "me",
    "my",
    "our",
    "remember",
    "recall",
    "please",
]);
function compactMemoryQuery(prompt) {
    const words = prompt
        .toLowerCase()
        .split(/[^\p{L}\p{N}_-]+/u)
        .map((word) => word.trim())
        .filter((word) => word.length > 2 && !PREFETCH_STOP_WORDS.has(word));
    const unique = Array.from(new Set(words)).slice(0, 6);
    return unique.length > 0 ? unique.join(" ") : null;
}
function pushUnique(values, value) {
    const trimmed = value?.trim();
    if (!trimmed)
        return;
    if (!values.some((existing) => existing.toLowerCase() === trimmed.toLowerCase()))
        values.push(trimmed);
}
export function classifyMemoryPrefetchPrompt(prompt) {
    const text = prompt.trim();
    if (!text)
        return null;
    if (EXPLICIT_RECALL_RE.test(text))
        return "explicit-recall";
    if (text.length <= 500 && CONTINUATION_FRAGMENT_RE.test(text))
        return "continuation";
    return null;
}
export function buildMemoryPrefetchQueries(prompt, reason) {
    const queries = [];
    pushUnique(queries, compactMemoryQuery(prompt));
    if (MUSIC_CONTEXT_RE.test(prompt)) {
        pushUnique(queries, "music");
        pushUnique(queries, "music project");
        pushUnique(queries, "electronic music");
        pushUnique(queries, "artist next level");
    }
    if (reason === "explicit-recall" || PROJECT_CONTEXT_RE.test(prompt)) {
        pushUnique(queries, "user-context");
        pushUnique(queries, "project-context");
    }
    return queries.slice(0, 8);
}
export function buildMemoryPrefetchContextMessage(payload) {
    if (payload.facts.length === 0 && payload.transcriptResults.length === 0)
        return null;
    const reasonLabel = payload.reason === "explicit-recall" ? "explicit recall cue" : "continuation cue";
    const lines = [
        `MEMORY PREFETCH (${reasonLabel}; hidden context for this turn):`,
        `Latest user prompt: ${JSON.stringify(truncate(payload.latestUserText, 160))}`,
    ];
    if (payload.facts.length > 0) {
        lines.push("Durable facts:");
        for (const fact of payload.facts.slice(0, 5)) {
            const tags = fact.tags ? ` tags=${fact.tags}` : "";
            lines.push(`- [#${fact.factId} trust=${fact.trustScore.toFixed(2)}${tags}] ${truncate(fact.content, 260)}`);
        }
    }
    if (payload.transcriptResults.length > 0) {
        lines.push("Transcript hits:");
        for (const result of payload.transcriptResults.slice(0, 3)) {
            const excerpt = sanitizeMemoryTranscriptText(result.snippet || result.content);
            lines.push(`- turn ${result.turnId} ${result.role}${result.toolName ? `/${result.toolName}` : ""}: ${truncate(excerpt, 220)}`);
        }
    }
    if (payload.queries.length > 0)
        lines.push(`Searches used: ${payload.queries.join("; ")}`);
    lines.push("Use this naturally to resolve pronouns/continuations. If the latest prompt adds a durable detail, save it with save_memory/fact_store.");
    return {
        role: "user",
        content: applyTokenBudget(lines, MEMORY_PREFETCH_CHAR_BUDGET).join("\n"),
        timestamp: payload.timestamp ?? Date.now(),
    };
}
function latestUserText(messages) {
    for (let i = messages.length - 1; i >= 0; i--) {
        const message = messages[i];
        if (message?.role !== "user")
            continue;
        return contentText(message.content).trim();
    }
    return "";
}
function collectMemoryPrefetch(latestText, memory, factStore) {
    if (isFalsey(process.env.HAMR_MEMORY_PREFETCH))
        return null;
    const reason = classifyMemoryPrefetchPrompt(latestText);
    if (!reason)
        return null;
    const queries = buildMemoryPrefetchQueries(latestText, reason);
    const facts = [];
    const seenFacts = new Set();
    const transcriptResults = [];
    const seenTranscript = new Set();
    const addFact = (fact) => {
        if (seenFacts.has(fact.factId))
            return;
        seenFacts.add(fact.factId);
        facts.push(fact);
    };
    const addTranscript = (result) => {
        const key = `${result.sessionId}:${result.turnId}:${result.role}:${result.toolName ?? ""}:${result.content.slice(0, 80)}`;
        if (seenTranscript.has(key))
            return;
        seenTranscript.add(key);
        transcriptResults.push(result);
    };
    for (const query of queries) {
        if (factStore?.isAvailable && facts.length < 5) {
            for (const fact of factStore.searchFacts(query, 3))
                addFact(fact);
        }
        if (memory && transcriptResults.length < 3) {
            for (const result of memory.searchWithSnippets(query, 2))
                addTranscript(result);
        }
    }
    // For explicit "remember that thing" prompts, a low-information query may not
    // match. Recent durable facts are safer here than pretending memory is empty.
    if (reason === "explicit-recall" && facts.length === 0 && factStore?.isAvailable) {
        for (const fact of factStore.listRecentFacts(3, 0.0))
            addFact(fact);
    }
    if (facts.length === 0 && transcriptResults.length === 0)
        return null;
    return { reason, latestUserText: latestText, queries, facts, transcriptResults };
}
export function selectCompactionPolicy(options) {
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
            instructions: "Large local context: keep a compact structured handoff and recover deeper provenance from FTS5 only as needed.",
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
            instructions: "Small local context: carry only decisions/status plus search keys; call search_memory for details.",
        };
    }
    return {
        tier: "local-16k",
        contextWindow,
        keyLimit: 16,
        searchTermLimit: 3,
        resultsPerTerm: 1,
        snippetChars: 120,
        instructions: "Tiny local context: do not replay history. Use the manifest as a map, then recover one key at a time with search_memory.",
    };
}
const INTENT_RE = /\b(i'?ll|i will|let me|let'?s|going to|next,? i|next step|the fix|plan to|need to|then i)\b/i;
const EDIT_TOOLS = new Set(["edit", "write", "str_replace", "str_replace_editor", "create_file", "apply_patch"]);
const PATH_ARG_KEYS = ["file_path", "path", "filePath", "filename", "file"];
function assistantText(message) {
    return message.content
        .filter((part) => part.type === "text")
        .map((part) => part.text)
        .join("")
        .trim();
}
function toolCalls(message) {
    return message.content.filter((part) => part.type === "toolCall");
}
function argPath(args) {
    if (!args)
        return undefined;
    for (const key of PATH_ARG_KEYS) {
        const value = args[key];
        if (typeof value === "string" && value.trim())
            return value.trim();
    }
    return undefined;
}
function splitSentences(text) {
    return text
        .split(/(?<=[.!?])\s+|\n+/)
        .map((s) => s.trim())
        .filter(Boolean);
}
function truncate(text, max) {
    const clean = text.replace(/\s+/g, " ").trim();
    return clean.length > max ? `${clean.slice(0, max - 1)}…` : clean;
}
/** Parse a `git switch`/`git checkout [-b]` command into the target branch name. */
function parseBranch(command) {
    const match = command.match(/git\s+(?:switch|checkout)\s+(?:-[bc]\s+)?([\w./-]+)/);
    const name = match?.[1];
    if (!name || name === "-" || name.startsWith("-"))
        return undefined;
    return name;
}
/** Extract the first specific error line from tool-result text. */
function firstErrorLine(text) {
    for (const raw of text.split("\n")) {
        const line = raw.trim();
        if (line.length < 8 || line.length > 200)
            continue;
        if (/\b(error|failed|failure|exception|cannot|not found|undefined)\b/i.test(line)) {
            return truncate(line, 160);
        }
    }
    return undefined;
}
/** Backticked tokens are how assistants name the specific identifiers central to the work. */
function backtickedIdentifiers(text) {
    const out = [];
    for (const match of text.matchAll(/`([^`\n]{2,60})`/g)) {
        const token = match[1].trim();
        // Keep specific identifiers/paths; drop prose-y multi-word backtick spans.
        if (token && !/\s/.test(token))
            out.push(token);
    }
    return out;
}
export function extractSurvivalData(messages, policy = selectCompactionPolicy({ cloud: false, contextWindow: 16_384 })) {
    const assistants = [];
    const orderedCalls = [];
    const resultsByCallId = new Map();
    let firstUserText = "";
    for (const message of messages) {
        if (message.role === "user") {
            const text = contentText(message.content).trim();
            if (!firstUserText && text)
                firstUserText = text;
        }
        else if (message.role === "assistant") {
            const assistant = message;
            assistants.push(assistant);
            for (const call of toolCalls(assistant))
                orderedCalls.push(call);
        }
        else if (message.role === "toolResult") {
            const result = message;
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
        if (intent)
            next = truncate(intent, 200);
    }
    if (!next)
        next = "(no explicit next action recorded)";
    // Status: ground truth from tool calls/results.
    const modified = new Set();
    let lastBash;
    let branch;
    for (const call of orderedCalls) {
        if (EDIT_TOOLS.has(call.name)) {
            const path = argPath(call.arguments);
            if (path)
                modified.add(path);
        }
        else if (call.name === "bash" && typeof call.arguments?.command === "string") {
            const command = call.arguments.command;
            lastBash = { command, result: resultsByCallId.get(call.id) };
            const parsed = parseBranch(command);
            if (parsed)
                branch = parsed;
        }
    }
    const status = [];
    if (modified.size > 0)
        status.push(`modified: ${Array.from(modified).slice(0, 8).join(", ")}`);
    if (lastBash) {
        const outcome = lastBash.result ? (lastBash.result.isError ? "failed" : "succeeded") : "result unknown";
        status.push(`last command: \`${truncate(lastBash.command, 100)}\` (${outcome})`);
    }
    if (branch)
        status.push(`branch: ${branch}`);
    // Keys: specific terms that each recover something lossless from FTS5.
    const keys = [];
    const provenance = [];
    const seen = new Set();
    const add = (candidate) => {
        if (keys.length >= policy.keyLimit || !candidate)
            return;
        const value = candidate.trim();
        if (!value || value.length > 160 || seen.has(value.toLowerCase()))
            return;
        seen.add(value.toLowerCase());
        keys.push(value);
    };
    const addProvenance = (candidate) => {
        if (!candidate || provenance.length >= 12)
            return;
        const value = candidate.trim();
        if (!value || provenance.some((p) => p.toLowerCase() === value.toLowerCase()))
            return;
        provenance.push(value);
    };
    // 1. Exact error strings being fought.
    for (const { isError, text } of resultsByCallId.values()) {
        if (keys.length >= policy.keyLimit)
            break;
        if (isError || /\b(error|failed|exception)\b/i.test(text))
            add(firstErrorLine(text));
    }
    // 2. Specific file paths actively being worked in.
    for (const call of orderedCalls) {
        if (keys.length >= policy.keyLimit)
            break;
        const path = argPath(call.arguments);
        add(path);
        addProvenance(path);
    }
    // 3. Specific identifiers / decisions named in assistant text.
    for (const assistant of assistants) {
        if (keys.length >= policy.keyLimit)
            break;
        for (const id of backtickedIdentifiers(assistantText(assistant)))
            add(id);
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
export function formatSurvivalManifest(data) {
    const lines = [
        "## Survival manifest (local-model compaction)",
        `Tier: ${data.tier} (${data.contextWindow.toLocaleString()} token window)`,
        `Task: ${data.task}`,
        `Recovery: ${data.instructions}`,
    ];
    if (data.status.length > 0) {
        lines.push("Status:", ...data.status.map((s) => `- ${s}`));
    }
    else {
        lines.push("Status: (no concrete state recorded)");
    }
    lines.push(`Next: ${data.next}`);
    if (data.keys.length > 0) {
        lines.push("Search keys (use search_memory to recover full detail from FTS5):");
        for (const key of data.keys)
            lines.push(`- ${key}`);
    }
    if (data.provenance.length > 0) {
        lines.push("Provenance anchors:");
        for (const item of data.provenance)
            lines.push(`- ${item}`);
    }
    return lines.join("\n");
}
/** Build the survival manifest string for a set of messages about to be discarded. */
export function buildSurvivalManifest(messages, policy) {
    return formatSurvivalManifest(extractSurvivalData(messages, policy));
}
/**
 * Cloud bookkeeping handoff: a lightweight manifest stored in FTS5 so the
 * resumed (LLM-summarized) session still has structured search hooks. Cloud
 * models keep pi's default compaction, so this never overrides the summary.
 */
function storeCompactionHandoff(memory, ctx, event) {
    const manifest = memory.handoff(ctx.sessionManager.getSessionId());
    memory.store({
        sessionId: ctx.sessionManager.getSessionId(),
        turnId: 0,
        role: "tool",
        toolName: "compaction_handoff",
        content: JSON.stringify({
            task: "Compaction handoff from pi auto-compaction",
            manifest,
            branchEntries: event.branchEntries?.length ?? 0,
        }, null, 2),
        domainTags: ["hamr", "compaction"],
    });
}
/**
 * Memory extension: FTS5 memory tools (search/save/handoff), message storage,
 * a two-path compaction strategy, and the turn counter. Orthogonal to session
 * topology — purely about persistence.
 */
export const hamrMemoryExtension = async (pi) => {
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
        if (!memory)
            return; // FTS5 unavailable → silent no-op, pi handles it.
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
        const memory = getMemory(ctx);
        const factStore = getFactStore(ctx);
        let messages = event.messages;
        let didPrefetch = false;
        // Cue-triggered prefetch is on by default and independent of broad auto-inject.
        // It fixes cold-resume fragments like: "the genre is deconstructed club".
        const latestText = latestUserText(event.messages);
        const prefetch = collectMemoryPrefetch(latestText, memory, factStore);
        const prefetchMessage = prefetch ? buildMemoryPrefetchContextMessage(prefetch) : null;
        if (prefetchMessage) {
            messages = [...messages, prefetchMessage];
            didPrefetch = true;
        }
        const prefetchOnly = () => (didPrefetch ? { messages } : undefined);
        // ── Opt-in gate: skip broad auto-injection unless HAMR_MEMORY_AUTO_INJECT is truthy ──
        if (!isTruthy(process.env.HAMR_MEMORY_AUTO_INJECT))
            return prefetchOnly();
        if (!memory)
            return prefetchOnly();
        const sessionId = ctx.sessionManager.getSessionId();
        // ── Only inject on resumed/handoff sessions (sessions with prior entries) ──
        if (!memory.hasSessionEntries(sessionId))
            return prefetchOnly();
        // ── One-shot: inject only once per session, never on every turn ──
        if (injectedSessions.has(sessionId))
            return prefetchOnly();
        let index = memory.buildMemoryIndex();
        // Append fact store status to the memory index
        if (factStore?.isAvailable) {
            const fc = factStore.getFactCount();
            const fsLine = fc > 0
                ? `\n[FactStore: ${fc} durable facts with entity resolution & trust scoring. Use fact_store to query, fact_feedback to rate.]`
                : `\n[FactStore: active, empty. Use fact_store(action='add') to persist cross-session knowledge.]`;
            index = index ? `${index}${fsLine}` : fsLine;
        }
        if (!index)
            return prefetchOnly();
        const survival = memory.getLatestByDomainTag("survival", sessionId);
        const survivalManifest = survival?.content ?? null;
        const provider = ctx.model?.provider;
        const cloud = !provider || isCloudProvider(config, provider);
        // Cloud providers: skip broad auto-injection entirely unless a survival
        // manifest from a prior local compaction exists. Cloud models rely on
        // proper LLM compaction, not broad FTS5 context injection.
        if (cloud && !survivalManifest)
            return prefetchOnly();
        const policy = selectCompactionPolicy({ cloud, contextWindow: ctx.model?.contextWindow });
        const terms = memory.getSuggestedSearchTerms();
        const autoResults = [];
        for (const term of terms.slice(0, policy.searchTermLimit)) {
            const results = memory.searchWithSnippets(term, policy.resultsPerTerm);
            if (results.length > 0) {
                autoResults.push(`// Search "${term}": ${results.length} results`, ...results.map((r) => `//   turn ${r.turnId} ${r.role}${r.toolName ? `/${r.toolName}` : ""}: ${sanitizeMemoryTranscriptText(r.snippet || r.content.slice(0, policy.snippetChars))}`));
            }
        }
        // ── De-duplicate against existing context messages ──
        const deduped = deduplicateResults(autoResults, messages);
        // ── Apply token budget cap ──
        const budgeted = applyTokenBudget(deduped, AUTO_INJECT_CHAR_BUDGET);
        // ── De-duplicate by content hash (skip if same content was already injected) ──
        const contextHash = hashContext(budgeted, index, survivalManifest);
        const prior = contextHashState.get(sessionId);
        if (prior === contextHash)
            return prefetchOnly();
        contextHashState.set(sessionId, contextHash);
        const message = buildMemoryContextMessage(budgeted, index, { survivalManifest });
        if (!message)
            return prefetchOnly();
        // Mark session as injected — one-shot only
        injectedSessions.add(sessionId);
        // APPEND memory context after existing messages to preserve prefix cache.
        return { messages: [...messages, message] };
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
//# sourceMappingURL=memory.js.map