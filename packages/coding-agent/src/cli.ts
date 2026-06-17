#!/usr/bin/env node
/**
 * CLI entry point for the refactored coding agent.
 * Uses main.ts with AgentSession and new mode modules.
 *
 * Test with: npx tsx src/cli-new.ts [args...]
 */
import { APP_NAME } from "./config.ts";
import { configureHttpDispatcher } from "./core/http-dispatcher.ts";
import { hamrBuiltinExtension } from "./hamr/extension.ts";
import { main } from "./main.ts";

process.title = APP_NAME;
process.env.PI_CODING_AGENT = "true";
process.emitWarning = (() => {}) as typeof process.emitWarning;

// Configure undici's global dispatcher before provider SDKs issue requests.
// Runtime settings are applied once SettingsManager has loaded global/project settings.
configureHttpDispatcher();

function normalizeHamrArgs(args: string[]): string[] {
	const [command, ...rest] = args;
	if (command === "chat") {
		return rest;
	}
	if (command !== "run") {
		return args;
	}

	const normalized: string[] = ["--print"];
	for (let i = 0; i < rest.length; i++) {
		const arg = rest[i];
		if (arg === "--task" && rest[i + 1] !== undefined) {
			normalized.push(rest[++i]);
		} else {
			normalized.push(arg);
		}
	}
	return normalized;
}

main(normalizeHamrArgs(process.argv.slice(2)), { extensionFactories: [hamrBuiltinExtension] });
