import type { ExtensionFactory } from "../../core/extensions/types.ts";
import { registerHandoffTool } from "../handoff/HandoffManager.ts";
import { getMemory, registerMemoryTools, setCurrentTurnId, storeMessage } from "../memory.ts";

/**
 * Memory extension: FTS5 memory tools (search/save/handoff), message storage,
 * a compaction handoff manifest, and the turn counter. Orthogonal to session
 * topology — purely about persistence.
 *
 * NOTE: the `context` handler auto-injects retrieved memory every turn, which is
 * an opinionated default that needs gating/opt-out — see issue #2.
 */
export const hamrMemoryExtension: ExtensionFactory = async (pi) => {
	registerMemoryTools(pi);
	registerHandoffTool(pi);

	// Store every completed message into FTS5 memory.
	pi.on("message_end", (event, ctx) => {
		storeMessage(ctx, event.message);
	});

	// When pi auto-compaction fires, create a handoff manifest from FTS5 memory
	// so the resumed session has structured context, not just replays.
	pi.on("session_before_compact", (event, ctx) => {
		const memory = getMemory(ctx);
		if (!memory) return;
		const manifest = memory.handoff();
		const sessionId = ctx.sessionManager.getSessionId();
		memory.store({
			sessionId,
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
	});

	// Context injection: auto-search memory and inject actual context for resumed
	// sessions. TODO(#2): this is unconditional and not opt-out — gate it.
	pi.on("context", (event, ctx) => {
		const memory = getMemory(ctx);
		if (!memory) return;
		const index = memory.buildMemoryIndex();
		if (!index) return;

		const terms = memory.getSuggestedSearchTerms();
		const autoResults: string[] = [];
		for (const term of terms.slice(0, 5)) {
			const results = memory.searchWithSnippets(term, 3);
			if (results.length > 0) {
				autoResults.push(
					`// Search "${term}": ${results.length} results`,
					...results.map(
						(r) =>
							`//   turn ${r.turnId} ${r.role}${r.toolName ? `/${r.toolName}` : ""}: ${r.snippet || r.content.slice(0, 120)}`,
					),
				);
			}
		}

		const contextParts = [
			`Hamr FTS5 memory index is available. Use search_memory for details before rereading broad history.`,
		];
		if (autoResults.length > 0) {
			contextParts.push(`\nAuto-retrieved context from prior sessions:\n${autoResults.join("\n")}`);
		}
		contextParts.push(`\n${index}`);

		return {
			messages: [
				{
					role: "user",
					content: contextParts.join("\n"),
					timestamp: Date.now(),
				},
				...event.messages,
			],
		};
	});

	// Advance the memory turn counter at the end of each turn.
	pi.on("turn_end", (event) => {
		setCurrentTurnId(event.turnIndex + 1);
	});
};
