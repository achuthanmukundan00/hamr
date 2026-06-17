#!/usr/bin/env node
/**
 * hamr CLI compatibility wrapper.
 *
 * The app runtime is pi's coding-agent main() with Hamr package metadata,
 * themes, local-model parsing, memory, and orchestration hooks layered in.
 */

import { main as runCodingAgent } from "@hamr/coding-agent";

function normalizeArgs(args: string[]): string[] {
	const [command, ...rest] = args;
	if (command === "chat") {
		return rest;
	}
	if (command !== "run") {
		return args;
	}

	const normalized: string[] = [];
	let task: string | undefined;
	for (let i = 0; i < rest.length; i++) {
		const arg = rest[i]!;
		if (arg === "--task") {
			task = rest[++i];
		} else if (arg.startsWith("--task=")) {
			task = arg.slice("--task=".length);
		} else {
			normalized.push(arg);
		}
	}

	if (!task) {
		const positional = normalized.filter((arg) => !arg.startsWith("-"));
		task = positional.join(" ").trim();
		for (const arg of positional) {
			const index = normalized.indexOf(arg);
			if (index >= 0) normalized.splice(index, 1);
		}
	}

	return task ? [...normalized, "--print", task] : [...normalized, "--print"];
}

async function main(): Promise<void> {
	process.title = "hamr";
	process.env.HAMR_CODING_AGENT = "true";
	process.env.PI_CODING_AGENT = "true";
	await runCodingAgent(normalizeArgs(process.argv.slice(2)));
}

main().catch((err: Error) => {
	console.error("hamr:", err.message);
	process.exit(1);
});
