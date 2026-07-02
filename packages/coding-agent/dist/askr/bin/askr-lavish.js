#!/usr/bin/env node
import { spawn } from "node:child_process";

const args = process.argv.slice(2);
const preferred = process.env.ASKR_LAVISH_BIN || "lavish-axi";

run(preferred, args, true);

function run(command, commandArgs, allowNpxFallback) {
  const child = spawn(command, commandArgs, {
    stdio: "inherit",
    shell: process.platform === "win32",
  });

  child.on("error", (error) => {
    if (allowNpxFallback && error.code === "ENOENT") {
      run("npx", ["-y", "lavish-axi", ...args], false);
      return;
    }
    console.log(`error: failed to launch lavish-axi: ${error.message}`);
    console.log("help:");
    console.log("  1. Install lavish-axi globally (deterministic, works offline): npm install -g lavish-axi");
    console.log("  2. Ensure npm/npx is installed and network is reachable for npx fallback");
    console.log("  3. Fallback: run `npx -y lavish-axi --help` manually, or present the artifact directly to the user");
    console.log("note: npx fallback fetches live remote code on every invocation — not reproducible, not suitable for CI/offline.");
    process.exit(1);
  });

  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code ?? 1);
  });
}
