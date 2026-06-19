import type { AgentMessage } from "@hamr/agent";
import { describe, expect, it } from "vitest";
import { buildSurvivalManifest, selectCompactionPolicy } from "../src/hamr/extensions/memory.ts";

function user(text: string): AgentMessage {
	return { role: "user", content: text, timestamp: 0 } as AgentMessage;
}

function assistant(
	parts: Array<{ text?: string; tool?: { id: string; name: string; args: Record<string, unknown> } }>,
): AgentMessage {
	const content = parts.map((p) =>
		p.tool
			? { type: "toolCall" as const, id: p.tool.id, name: p.tool.name, arguments: p.tool.args }
			: { type: "text" as const, text: p.text ?? "" },
	);
	return { role: "assistant", content, timestamp: 0 } as unknown as AgentMessage;
}

function toolResult(toolCallId: string, toolName: string, text: string, isError = false): AgentMessage {
	return {
		role: "toolResult",
		toolCallId,
		toolName,
		content: [{ type: "text", text }],
		isError,
		timestamp: 0,
	} as unknown as AgentMessage;
}

describe("buildSurvivalManifest", () => {
	it("captures the verbatim task from the first user message", () => {
		const manifest = buildSurvivalManifest([
			user("Add retry logic to the relay provider so transient 502s are retried twice."),
			assistant([{ text: "Sure, I'll start by reading the provider." }]),
		]);
		expect(manifest).toContain("Task:");
		expect(manifest).toContain("Add retry logic to the relay provider");
	});

	it("lists files modified via edit/write tool calls in status", () => {
		const manifest = buildSurvivalManifest([
			user("Fix the footer."),
			assistant([
				{ text: "Editing the footer." },
				{ tool: { id: "t1", name: "edit", args: { file_path: "src/footer.ts" } } },
				{ tool: { id: "t2", name: "write", args: { path: "src/footer.test.ts" } } },
			]),
		]);
		expect(manifest).toContain("src/footer.ts");
		expect(manifest).toContain("src/footer.test.ts");
	});

	it("records the last command and whether it failed", () => {
		const manifest = buildSurvivalManifest([
			user("Run the tests."),
			assistant([{ tool: { id: "b1", name: "bash", args: { command: "npm test" } } }]),
			toolResult("b1", "bash", "1 failing\nError: assertion failed", true),
		]);
		expect(manifest).toContain("npm test");
		expect(manifest).toContain("failed");
	});

	it("records a successful command result", () => {
		const manifest = buildSurvivalManifest([
			user("Build it."),
			assistant([{ tool: { id: "b1", name: "bash", args: { command: "npm run build" } } }]),
			toolResult("b1", "bash", "Build complete", false),
		]);
		expect(manifest).toContain("npm run build");
		expect(manifest).toContain("succeeded");
	});

	it("extracts exact error strings from failing tool results as search keys", () => {
		const manifest = buildSurvivalManifest([
			user("Fix the crash."),
			assistant([{ tool: { id: "b1", name: "bash", args: { command: "node app.js" } } }]),
			toolResult("b1", "bash", "TypeError: cannot read properties of undefined (reading 'content')", true),
		]);
		expect(manifest).toContain("cannot read properties of undefined");
	});

	it("captures the planned next action from intent language in the last assistant message", () => {
		const manifest = buildSurvivalManifest([
			user("Investigate the bug."),
			assistant([{ text: "I read the file." }]),
			assistant([{ text: "The cause is clear. Next I'll patch parseBranch to handle git switch." }]),
		]);
		expect(manifest).toContain("Next:");
		expect(manifest).toContain("parseBranch");
	});

	it("surfaces backticked identifiers from assistant text as search keys", () => {
		const manifest = buildSurvivalManifest([
			user("Wire up the handler."),
			assistant([{ text: "The fix lives in the `session_before_compact` handler." }]),
		]);
		expect(manifest).toContain("session_before_compact");
	});

	it("stays compact and labeled even with sparse input", () => {
		const manifest = buildSurvivalManifest([user("Do the thing.")]);
		expect(manifest).toContain("Task:");
		expect(manifest).toContain("Next:");
		expect(manifest.length).toBeLessThan(2000);
	});

	it("labels the selected local compaction tier and recovery instruction", () => {
		const manifest = buildSurvivalManifest(
			[user("Finish the local model memory handoff."), assistant([{ text: "Next I'll patch `HolographicMemory`." }])],
			selectCompactionPolicy({ cloud: false, contextWindow: 32_768 }),
		);
		expect(manifest).toContain("Tier: local-32k");
		expect(manifest).toContain("Recovery:");
		expect(manifest).toContain("HolographicMemory");
	});

	it("uses the tier key limit for tiny local models", () => {
		const messages: AgentMessage[] = [
			user("Preserve lots of keys."),
			assistant([
				{ text: "Keep `alpha` `beta` `gamma` `delta` `epsilon` `zeta` `eta` `theta` `iota` `kappa`." },
				{ tool: { id: "e1", name: "edit", args: { file_path: "src/a.ts" } } },
				{ tool: { id: "e2", name: "edit", args: { file_path: "src/b.ts" } } },
				{ tool: { id: "e3", name: "edit", args: { file_path: "src/c.ts" } } },
				{ tool: { id: "e4", name: "edit", args: { file_path: "src/d.ts" } } },
				{ tool: { id: "e5", name: "edit", args: { file_path: "src/e.ts" } } },
				{ tool: { id: "e6", name: "edit", args: { file_path: "src/f.ts" } } },
				{ tool: { id: "e7", name: "edit", args: { file_path: "src/g.ts" } } },
			]),
		];
		const manifest = buildSurvivalManifest(messages, selectCompactionPolicy({ cloud: false, contextWindow: 16_384 }));
		const keyLines = manifest.split("\n").filter((line) => line.startsWith("- "));
		expect(manifest).toContain("Tier: local-16k");
		expect(keyLines.length).toBeGreaterThan(8);
	});
});
