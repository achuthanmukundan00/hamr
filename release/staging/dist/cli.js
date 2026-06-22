#!/usr/bin/env node
/**
 * CLI entry point for the refactored coding agent.
 * Uses main.ts with AgentSession and new mode modules.
 *
 * Test with: npx tsx src/cli-new.ts [args...]
 */
import { APP_NAME } from "./config.js";
import { configureHttpDispatcher } from "./core/http-dispatcher.js";
import { hamrDefaultExtensions } from "./hamr/extensions/index.js";
import { main } from "./main.js";
process.title = APP_NAME;
process.env.HAMR_CODING_AGENT = "true";
process.env.PI_CODING_AGENT = "true";
process.emitWarning = (() => { });
// Configure undici's global dispatcher before provider SDKs issue requests.
// Runtime settings are applied once SettingsManager has loaded global/project settings.
configureHttpDispatcher();
function normalizeHamrArgs(args) {
    const [command, ...rest] = args;
    if (command === "chat") {
        return rest;
    }
    if (command !== "run") {
        return args;
    }
    const normalized = ["--print"];
    for (let i = 0; i < rest.length; i++) {
        const arg = rest[i];
        if (arg === "--task" && rest[i + 1] !== undefined) {
            normalized.push(rest[++i]);
        }
        else {
            normalized.push(arg);
        }
    }
    return normalized;
}
main(normalizeHamrArgs(process.argv.slice(2)), { extensionFactories: hamrDefaultExtensions });
//# sourceMappingURL=cli.js.map