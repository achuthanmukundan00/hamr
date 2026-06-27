import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { Text } from "@hamr/tui";
import { Type } from "typebox";
import { defineTool } from "../core/extensions/types.js";
import { contentText, fileHints, getAssistantText } from "./helpers.js";
import { FactStore } from "./memory/FactStore.js";
import { stripFtsMarks } from "./memory/fts-marks.js";
import { HolographicMemory } from "./memory/HolographicMemory.js";
import { loadNodeSqliteDatabase } from "./store/node-sqlite-adapter.js";
import { loadBetterSqlite3 } from "./store/sqlite-loader.js";
let memoryHandle;
/** Paths where SQLite init failed, so we don't retry on every call. */
const failedPaths = new Set();
let currentTurnId = 0;
export function setCurrentTurnId(id) {
    currentTurnId = id;
}
export function getCurrentTurnId() {
    return currentTurnId;
}
/**
 * Resolve the memory database path.
 *
 * Priority:
 *   1. HAMR_MEMORY_DB env var — absolute or relative (relative to cwd)
 *   2. ~/.hamr/memory.sqlite — centralized, one DB for all projects
 *
 * Previously defaulted to `<cwd>/.hamr/memory.sqlite` which littered a
 * separate DB in every project directory. Centralizing to ~/.hamr means
 * all sessions share one memory store, and the agent can find facts from
 * any project.
 */
function memoryPath(cwd) {
    if (process.env.HAMR_MEMORY_DB) {
        const envPath = process.env.HAMR_MEMORY_DB;
        // If the env var is a relative path, resolve it against cwd
        if (envPath.startsWith("/") || envPath.startsWith("~")) {
            return envPath.replace(/^~/, homedir());
        }
        return join(cwd, envPath);
    }
    // Centralized: ~/.hamr/memory.sqlite — one memory store for all projects
    return join(homedir(), ".hamr", "memory.sqlite");
}
export function getMemory(ctx) {
    const handle = getMemoryHandle(ctx.cwd);
    return handle?.memory;
}
function getMemoryHandle(cwd) {
    const path = memoryPath(cwd);
    if (memoryHandle?.path === path)
        return memoryHandle;
    if (failedPaths.has(path))
        return undefined;
    // Try better-sqlite3 first (native C++ addon), fall back to node:sqlite (Node 24+).
    // node:sqlite ships inside the runtime with FTS5 enabled — no compilation needed.
    let Database = loadBetterSqlite3();
    if (!Database) {
        Database = loadNodeSqliteDatabase();
        if (Database) {
            console.warn("[hamr] Using node:sqlite (built-in) as SQLite backend. better-sqlite3 unavailable.");
        }
    }
    if (!Database) {
        failedPaths.add(path);
        console.error("[hamr] SQLite unavailable: better-sqlite3 AND node:sqlite both failed. " +
            "Memory (search_memory / save_memory / fact_store) will be disabled. " +
            "Install build tools for better-sqlite3 or upgrade to Node 24+ for built-in sqlite. " +
            "Set HAMR_MEMORY_DB env var to override the db path (default: ~/.hamr/memory.sqlite).");
        return undefined;
    }
    // ── Migration from v0.7.0: copy old <cwd>/.hamr/memory.sqlite → ~/.hamr/memory.sqlite ──
    // The old default was <cwd>/.hamr/memory.sqlite which created one DB per project.
    // The new default is ~/.hamr/memory.sqlite (centralized). On first access to the
    // new path, copy any existing DB from the old project-scoped location so users
    // don't lose their accumulated memory and facts.
    if (!process.env.HAMR_MEMORY_DB && !existsSync(path)) {
        const oldPath = join(cwd, ".hamr", "memory.sqlite");
        if (existsSync(oldPath)) {
            try {
                mkdirSync(dirname(path), { recursive: true });
                copyFileSync(oldPath, path);
                console.warn(`[hamr] Migrated memory DB from ${oldPath} → ${path}`);
            }
            catch (err) {
                console.warn(`[hamr] Could not migrate old memory DB from ${oldPath}:`, err);
            }
        }
    }
    // Close old connection before opening new one
    if (memoryHandle) {
        try {
            memoryHandle.memory.db?.close();
        }
        catch {
            // ignore close errors
        }
        memoryHandle = undefined;
    }
    try {
        mkdirSync(dirname(path), { recursive: true });
        const db = new Database(path);
        db.pragma("journal_mode = WAL");
        db.pragma("foreign_keys = ON");
        db.exec(`
			CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
				turn_id UNINDEXED,
				session_id UNINDEXED,
				role UNINDEXED,
				tool_name UNINDEXED,
				file_paths UNINDEXED,
				content,
				domain_tags UNINDEXED
			);
		`);
        const holographic = new HolographicMemory(db);
        const factStore = new FactStore(db);
        memoryHandle = { path, memory: holographic, factStore };
        return memoryHandle;
    }
    catch (err) {
        console.error(`[hamr] Failed to initialize SQLite memory at ${path}:`, err);
        failedPaths.add(path);
        return undefined;
    }
}
/** Get the cross-session structured FactStore (entity resolution, trust scoring, HRR). */
export function getFactStore(ctx) {
    const handle = getMemoryHandle(ctx.cwd);
    return handle?.factStore;
}
export function buildAssistantMemoryContent(message) {
    const parts = [];
    const text = sanitizeMemoryTranscriptText(getAssistantText(message)).trim();
    if (text)
        parts.push(text);
    for (const block of message.content) {
        if (block.type !== "toolCall")
            continue;
        const call = block;
        parts.push(`tool_call ${call.name} ${JSON.stringify(call.arguments)}`);
    }
    return parts.join("\n");
}
export function sanitizeMemoryTranscriptText(text) {
    return stripFtsMarks(text)
        .replace(/<think\b[^>]*>[\s\S]*?<\/think>/gi, "")
        .replace(/<thinking\b[^>]*>[\s\S]*?<\/thinking>/gi, "")
        .replace(/<function=([^>\s]+)>/gi, "tool_call $1 ")
        .replace(/<parameter=([^>\s]+)>/gi, "$1=")
        .replace(/<\/parameter>/gi, " ")
        .replace(/<\/function>/gi, " ")
        .replace(/<\/?tool_call>/gi, " ")
        .replace(/<\/?function_calls>/gi, " ")
        .replace(/<\/?function_call>/gi, " ")
        .replace(/[ \t]+\n/g, "\n")
        .replace(/\n{3,}/g, "\n\n")
        .trim();
}
export function storeMessage(ctx, message) {
    const memory = getMemory(ctx);
    if (!memory)
        return;
    if (message.role === "user") {
        const text = contentText(message.content);
        if (!text.trim())
            return;
        memory.store({
            sessionId: ctx.sessionManager.getSessionId(),
            turnId: currentTurnId,
            role: "user",
            content: text,
            filePaths: fileHints(text),
            domainTags: ["hamr"],
        });
        return;
    }
    if (message.role === "assistant") {
        const text = buildAssistantMemoryContent(message);
        if (!text.trim())
            return;
        memory.store({
            sessionId: ctx.sessionManager.getSessionId(),
            turnId: currentTurnId,
            role: "assistant",
            content: text,
            filePaths: fileHints(text),
            domainTags: ["hamr"],
        });
        return;
    }
    if (message.role === "toolResult") {
        const result = message;
        const text = contentText(result.content);
        if (!text.trim())
            return;
        memory.store({
            sessionId: ctx.sessionManager.getSessionId(),
            turnId: currentTurnId,
            role: "tool",
            toolName: result.toolName,
            content: text,
            filePaths: fileHints(text),
            domainTags: ["hamr"],
        });
    }
}
export function registerMemoryTools(pi) {
    pi.registerTool(defineTool({
        name: "search_memory",
        label: "Search memory",
        description: "Search Hamr's local FTS5 memory for prior turns, tool outputs, files, and handoff facts.",
        promptSnippet: "Use search_memory to recover relevant prior context without rereading the whole conversation.",
        parameters: Type.Object({
            query: Type.String({ description: "FTS5 query text, for example an error, file path, or feature name." }),
            limit: Type.Optional(Type.Number({ description: "Maximum results to return. Default 5." })),
        }),
        renderResult: (result, options, theme) => {
            const details = result.details;
            const count = details?.count ?? 0;
            if (!options.expanded) {
                return new Text(theme.fg("dim", `${count} result${count !== 1 ? "s" : ""}`), 0, 0);
            }
            const output = contentText(result.content);
            return new Text(theme.fg("toolOutput", output), 0, 0);
        },
        execute: async (_toolCallId, params, _signal, _onUpdate, ctx) => {
            const memory = getMemory(ctx);
            if (!memory)
                return {
                    content: [
                        {
                            type: "text",
                            text: "⚠️ FTS5 memory is unavailable. Hamr cannot store or search session history. " +
                                "Likely causes: (1) better-sqlite3 native addon failed to compile — run `npm rebuild -g better-sqlite3` " +
                                "or install build tools (Xcode CLT / build-essential), (2) Node version < 24 and node:sqlite fallback " +
                                "not available, (3) Permission error writing to ~/.hamr/memory.sqlite. " +
                                "Check the terminal output for [hamr] warnings during startup.",
                        },
                    ],
                    details: {},
                };
            const results = memory.searchWithSnippets(params.query, Math.min(params.limit ?? 5, 20));
            const text = results.length === 0
                ? "No memory results."
                : results
                    .map((result, index) => {
                    const excerpt = sanitizeMemoryTranscriptText(result.snippet || result.content.slice(0, 500));
                    return `${index + 1}. turn ${result.turnId} ${result.role}${result.toolName ? `/${result.toolName}` : ""}\n${excerpt}`;
                })
                    .join("\n\n");
            return { content: [{ type: "text", text }], details: { count: results.length } };
        },
    }));
    pi.registerTool(defineTool({
        name: "save_memory",
        label: "Save memory",
        description: "Save a durable fact, decision, or handoff note into Hamr's local FTS5 memory.",
        parameters: Type.Object({
            content: Type.String({ description: "The fact, decision, or handoff note to store." }),
            tags: Type.Optional(Type.Array(Type.String({ description: "Optional tags." }))),
        }),
        execute: async (_toolCallId, params, _signal, _onUpdate, ctx) => {
            const memory = getMemory(ctx);
            if (!memory)
                return {
                    content: [
                        {
                            type: "text",
                            text: "⚠️ FTS5 memory is unavailable — cannot save. See search_memory error for troubleshooting.",
                        },
                    ],
                    details: {},
                };
            const errCountBefore = memory.storeErrorCount;
            memory.store({
                sessionId: ctx.sessionManager.getSessionId(),
                turnId: currentTurnId,
                role: "tool",
                toolName: "save_memory",
                content: params.content,
                filePaths: fileHints(params.content),
                domainTags: ["hamr", ...(params.tags ?? [])],
            });
            const tags = params.tags ?? [];
            const tagLine = tags.length > 0 ? `\nTags: ${tags.join(", ")}` : "";
            const preview = params.content.length > 300 ? `${params.content.slice(0, 300)}…` : params.content;
            if (memory.storeErrorCount > errCountBefore) {
                return {
                    content: [
                        {
                            type: "text",
                            text: `⚠️ Failed to save to Hamr memory. Store error count: ${memory.storeErrorCount}. Check ~/.hamr/memory.sqlite and better-sqlite3 native addon.`,
                        },
                    ],
                    details: { tags, storedLength: params.content.length, failed: true },
                };
            }
            // Mirror to cross-session FactStore when content is statement-like
            // (only after transcript store succeeded)
            const factStore = getFactStore(ctx);
            let factResult = "";
            if (factStore?.isAvailable && params.content.length > 30) {
                const factId = factStore.addFact(params.content, (params.tags ?? []).join(","));
                if (factId !== null && factId > 0) {
                    factResult = ` (also stored as durable fact #${factId})`;
                }
            }
            return {
                content: [{ type: "text", text: `📝 Saved to Hamr memory:\n${preview}${tagLine}${factResult}` }],
                details: { tags, storedLength: params.content.length },
            };
        },
    }));
    registerFactStoreTools(pi);
    pi.registerTool(defineTool({
        name: "handoff_memory",
        label: "Memory handoff",
        description: "Build a structured handoff manifest from Hamr's FTS5 memory for another agent or future turn.",
        parameters: Type.Object({}),
        renderResult: (result, options, theme) => {
            const handoff = result.details;
            if (!options.expanded) {
                const count = handoff?.entryCount ?? 0;
                return new Text(theme.fg("dim", `${count} entr${count !== 1 ? "ies" : "y"} · ${handoff?.turnCount ?? 0} turns`), 0, 0);
            }
            const output = contentText(result.content);
            return new Text(theme.fg("toolOutput", output), 0, 0);
        },
        execute: async (_toolCallId, _params, _signal, _onUpdate, ctx) => {
            const memory = getMemory(ctx);
            if (!memory)
                return {
                    content: [
                        {
                            type: "text",
                            text: "⚠️ FTS5 memory is unavailable — cannot build handoff. See search_memory error for troubleshooting.",
                        },
                    ],
                    details: {},
                };
            const handoff = memory.handoff(ctx.sessionManager.getSessionId());
            const lines = [
                `📋 Hamr handoff manifest`,
                `Session: ${handoff.sessionId || ctx.sessionManager.getSessionId()}`,
                `Turns: ${handoff.turnCount}, entries: ${handoff.entryCount}`,
            ];
            if (handoff.filesTouched.length > 0)
                lines.push(`Files touched: ${handoff.filesTouched.join(", ")}`);
            if (handoff.domainTags.length > 0)
                lines.push(`Tags: ${handoff.domainTags.join(", ")}`);
            if (handoff.suggestedSearchTerms.length > 0)
                lines.push(`Suggested searches: ${handoff.suggestedSearchTerms.join(", ")}`);
            if (handoff.keyFindings.length > 0) {
                lines.push(`Key findings:\n${handoff.keyFindings.map((f) => `  - ${f}`).join("\n")}`);
            }
            return { content: [{ type: "text", text: lines.join("\n") }], details: handoff };
        },
    }));
}
/**
 * Register structured fact store tools (fact_store and fact_feedback).
 * The fact store provides cross-session durable knowledge with entity
 * resolution, trust scoring, and HRR-based compositional queries.
 */
export function registerFactStoreTools(pi) {
    // Helper: produce a consistent error result with required `details`.
    const toolError = (text) => ({
        content: [{ type: "text", text }],
        isError: true,
        details: {},
    });
    pi.registerTool(defineTool({
        name: "fact_store",
        label: "Fact store",
        description: "Structured cross-session fact store. Store durable facts with entity resolution and trust scoring. " +
            "Use fact_store for deep recall and compositional queries. " +
            "ACTIONS: add, search, probe, related, reason. " +
            "ALWAYS probe or reason before answering questions from memory.",
        promptSnippet: "Use fact_store to store and query durable structured knowledge across sessions. Entities are auto-extracted.",
        parameters: Type.Object({
            action: Type.Union([
                Type.Literal("add"),
                Type.Literal("search"),
                Type.Literal("probe"),
                Type.Literal("related"),
                Type.Literal("reason"),
            ]),
            content: Type.Optional(Type.String({ description: "Fact content (required for 'add')." })),
            query: Type.Optional(Type.String({ description: "Search query (required for 'search')." })),
            entity: Type.Optional(Type.String({ description: "Entity name for 'probe'/'related'." })),
            entities: Type.Optional(Type.Array(Type.String(), { description: "Entity names for 'reason' (AND semantics)." })),
            tags: Type.Optional(Type.String({ description: "Comma-separated tags." })),
            limit: Type.Optional(Type.Number({ description: "Max results (default: 10)." })),
        }),
        renderResult: (result, _options, theme) => {
            const output = contentText(result.content);
            return new Text(theme.fg("toolOutput", output), 0, 0);
        },
        execute: async (_toolCallId, params, _signal, _onUpdate, ctx) => {
            const factStore = getFactStore(ctx);
            if (!factStore?.isAvailable) {
                return toolError("Fact store is unavailable.");
            }
            const action = params.action;
            const limit = Math.min(params.limit ?? 10, 20);
            try {
                switch (action) {
                    case "add": {
                        if (!params.content)
                            return toolError("Missing 'content' for add.");
                        const factId = factStore.addFact(params.content, params.tags ?? "");
                        if (factId === null || factId <= 0)
                            return toolError("Failed to add fact.");
                        const fact = factStore.getFact(factId);
                        const entityLine = fact?.entities?.length ? ` | entities: ${fact.entities.join(", ")}` : "";
                        return {
                            content: [{ type: "text", text: `📌 Fact #${factId} stored${entityLine}` }],
                            details: { factId, entities: fact?.entities ?? [] },
                        };
                    }
                    case "search": {
                        if (!params.query)
                            return toolError("Missing 'query' for search.");
                        const results = factStore.searchFacts(params.query, limit);
                        if (results.length === 0)
                            return { content: [{ type: "text", text: "No facts found." }], details: { count: 0 } };
                        const lines = results.map((r) => `  [#${r.factId}] trust=${r.trustScore.toFixed(2)} ${r.content.slice(0, 180)}`);
                        return {
                            content: [{ type: "text", text: `${results.length} facts:\n${lines.join("\n")}` }],
                            details: { count: results.length },
                        };
                    }
                    case "probe": {
                        if (!params.entity)
                            return toolError("Missing 'entity' for probe.");
                        const results = factStore.probe(params.entity, limit);
                        if (results.length === 0)
                            return {
                                content: [{ type: "text", text: `No facts found for entity "${params.entity}".` }],
                                details: { count: 0 },
                            };
                        const lines = results.map((r) => `  [#${r.factId}] score=${(r.score ?? 0).toFixed(2)} trust=${r.trustScore.toFixed(2)} ${r.content.slice(0, 180)}`);
                        return {
                            content: [
                                { type: "text", text: `${results.length} facts about "${params.entity}":\n${lines.join("\n")}` },
                            ],
                            details: { count: results.length },
                        };
                    }
                    case "related": {
                        if (!params.entity)
                            return toolError("Missing 'entity' for related.");
                        const results = factStore.related(params.entity, limit);
                        if (results.length === 0)
                            return {
                                content: [{ type: "text", text: `No facts related to "${params.entity}".` }],
                                details: { count: 0 },
                            };
                        const lines = results.map((r) => `  [#${r.factId}] score=${(r.score ?? 0).toFixed(2)} trust=${r.trustScore.toFixed(2)} ${r.content.slice(0, 180)}`);
                        return {
                            content: [
                                { type: "text", text: `${results.length} facts related to "${params.entity}":\n${lines.join("\n")}` },
                            ],
                            details: { count: results.length },
                        };
                    }
                    case "reason": {
                        const entities = params.entities ?? [];
                        if (entities.length === 0)
                            return toolError("Missing 'entities' list for reason.");
                        const results = factStore.reason(entities, limit);
                        if (results.length === 0)
                            return {
                                content: [{ type: "text", text: `No facts found connecting all: ${entities.join(", ")}.` }],
                                details: { count: 0 },
                            };
                        const lines = results.map((r) => `  [#${r.factId}] score=${(r.score ?? 0).toFixed(2)} trust=${r.trustScore.toFixed(2)} ${r.content.slice(0, 180)}`);
                        return {
                            content: [
                                {
                                    type: "text",
                                    text: `${results.length} facts connecting ${entities.join(" & ")}:\n${lines.join("\n")}`,
                                },
                            ],
                            details: { count: results.length },
                        };
                    }
                    default:
                        return toolError(`Unknown action: ${action}`);
                }
            }
            catch (err) {
                return toolError(`Fact store error: ${err instanceof Error ? err.message : String(err)}`);
            }
        },
    }));
    pi.registerTool(defineTool({
        name: "fact_feedback",
        label: "Fact feedback",
        description: "Rate a fact after using it. Mark 'helpful' if accurate, 'unhelpful' if outdated. Good facts rise (trust +0.05), bad facts sink (trust −0.10).",
        parameters: Type.Object({
            action: Type.Union([Type.Literal("helpful"), Type.Literal("unhelpful")]),
            fact_id: Type.Number({ description: "The fact ID to rate." }),
        }),
        renderResult: (result, _options, theme) => {
            const output = contentText(result.content);
            return new Text(theme.fg("toolOutput", output), 0, 0);
        },
        execute: async (_toolCallId, params, _signal, _onUpdate, ctx) => {
            const factStore = getFactStore(ctx);
            if (!factStore?.isAvailable) {
                return toolError("Fact store is unavailable.");
            }
            try {
                const helpful = params.action === "helpful";
                const result = factStore.recordFeedback(params.fact_id, helpful);
                if (!result) {
                    return toolError(`Fact #${params.fact_id} not found.`);
                }
                const direction = helpful ? "↑" : "↓";
                return {
                    content: [
                        {
                            type: "text",
                            text: `📊 Fact #${params.fact_id}: trust ${result.oldTrust.toFixed(2)} → ${result.newTrust.toFixed(2)} ${direction}`,
                        },
                    ],
                    details: result,
                };
            }
            catch (err) {
                return toolError(`Fact feedback error: ${err instanceof Error ? err.message : String(err)}`);
            }
        },
    }));
}
//# sourceMappingURL=memory.js.map