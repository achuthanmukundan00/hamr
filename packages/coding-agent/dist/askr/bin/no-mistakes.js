#!/usr/bin/env node
import { spawn } from "node:child_process";

const args = process.argv.slice(2);
const preferred = process.env.ASKR_NO_MISTAKES_BIN || "no-mistakes";

run(preferred, args, true);

function run(command, commandArgs, allowNpxFallback) {
  const child = spawn(command, commandArgs, {
    stdio: "inherit",
    shell: process.platform === "win32",
  });

  child.on("error", (error) => {
    if (allowNpxFallback && error.code === "ENOENT") {
      run("npx", ["-y", "@skaft/no-mistakes", ...args], false);
      return;
    }
    console.log(`error: failed to launch no-mistakes: ${error.message}`);
    console.log("help:");
    console.log("  1. Install no-mistakes globally: npm install -g @skaft/no-mistakes");
    console.log("  2. Follow the no-mistakes installation guide at https://github.com/kunchenguid/no-mistakes");
    console.log("  3. Fallback: run `npx -y @skaft/no-mistakes --help` manually");
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
