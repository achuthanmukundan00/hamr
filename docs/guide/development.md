# Developing Hamr

How to set up Hamr for local development and contribute.

## Prerequisites

- **Bun** ≥ 1.0 (required for `sexy-tui-rs` native addon)
- **Node.js** ≥ 18 (for tooling compatibility)
- **Git**

## Clone and Install

```bash
git clone git@github.com:skaft/hamr.git
cd hamr
bun install
```

## Build

```bash
bun run build
```

This compiles TypeScript to `dist/` using `tsgo` and copies theme JSON files.

## Verify (Full CI Gate)

```bash
bun verify
```

Runs in order:
1. `bun run typecheck` — `tsgo --noEmit`
2. `bun run lint` — `biome check src/`
3. `bun run build` — compiles to `dist/`
4. `bun test` — Jest (all test suites)

All four must pass. The verify command is the CI gate.

## Run Locally

```bash
bun run hamr -- [command] [args]
```

Examples:
```bash
bun run hamr --                    # Launch TUI
bun run hamr -- chat --plain       # Plain (non-TUI) chat
bun run hamr -- run --task "fix the build" --yes
bun run hamr -- doctor
```

## Project Structure

See [Architecture](/guide/architecture) for the full module map.

## Key Modules

| Module | Purpose | When to touch |
|--------|---------|---------------|
| `src/session/Session.ts` | Core turn loop, tool execution, recovery | Agent behavior changes |
| `src/session/formatting.ts` | Message construction, safety checks | Output formatting, preamble detection |
| `src/session/message-assembly.ts` | Budget guard, compaction | Context budget changes |
| `src/agent/context-budget.ts` | Token estimation, multi-stage compaction | Compaction strategy |
| `src/llm/client.ts` | HTTP client, streaming, parseSuccessResponse | Provider integration |
| `src/llm/parsers/` | Tool-call parsers (12 families) | New parser support |
| `src/tui/hamr-tui.ts` | TUI orchestration | UI behavior changes |
| `src/tui/components/event-feed.ts` | Feed rendering, cards, Markdown | Card rendering |
| `src/tui/components/status-bar.ts` | Status bar | Status info display |
| `src/tui/semantic-events.ts` | AgentEvent → UI card classifier | Event classification |
| `src/actions/ActionExecutor.ts` | Tool execution dispatch | Tool behavior |
| `src/actions/handlers/` | Tool handlers (bash, read, edit, etc.) | Tool implementation |

## Testing

```bash
bun test                          # All tests
bun run jest -- --testPathPattern="runner"  # Specific suite
bun run jest -- --verbose         # Verbose output
```

Test files live alongside source in `src/__tests__/`. Mock LLM clients for turn-loop tests — no live model server needed.

## Docs

```bash
bun run docs:dev                  # Live preview at localhost:5173
bun run docs:build                # Build for deployment
```

Docs are VitePress under `docs/`. The AGENTS.md at the repo root is the entry point for coding agents working on Hamr itself.

## Conventions

- **No Rust** — do not introduce Rust code (the TUI addon is pre-built)
- **No Python services** — Hamr is TypeScript-only
- **No database** — FTS5 SQLite for holographic memory is the only persistence
- **No web UI** — CLI and TUI only
- **Strict mode TypeScript** — prefer explicit return types, discriminated unions
- **Biome for lint/format** — run `bun run lint` before committing
