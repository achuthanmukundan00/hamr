#!/usr/bin/env node
/**
 * hamr — CLI entry point.
 *
 * pi handles: config loading, model resolution, session management,
 * auth, skills, prompt templates, extensions, trust management.
 *
 * Hamr adds: TUI rendering (via pi's TUI with Hamr themes),
 * local-model provider fallback, memory tools.
 */

import { createSession } from './session.js';

async function main() {
  const args = process.argv.slice(2);

  // pi reads .pi/config.toml, models.json, .env automatically.
  // Model resolution: --model flag > config default > provider default.
  // For now, let pi's defaults handle everything.
  const session = createSession({
    // TODO: wire config loading from .pi/ or .hamr/
    // pi's SDK handles this via ModelRegistry, AuthStorage, SettingsManager.
    // For MVP, pass model + tools explicitly or let pi discover.
  } as any);

  if (args[0] === 'run') {
    const task = args.slice(1).join(' ');
    const result = await session.prompt(task);
    process.exit(0);
  }

  // Interactive TUI — pi's TUI handles everything
  // TODO: boot pi's interactive mode with Hamr themes
  console.log('hamr chat — TUI mode coming soon (pi TUI + Hamr themes)');
  process.exit(0);
}

main().catch((err: Error) => {
  console.error('hamr:', err.message);
  process.exit(1);
});
