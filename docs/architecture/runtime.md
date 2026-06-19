# Runtime Architecture

Hamr started as a Pi fork and the runtime is still evolving. This page is a current map, not a frozen spec. File paths and boundaries will keep moving while the runtime is being built out.

## Current Map

| Area | Current Source |
| --- | --- |
| CLI entrypoints | [`packages/coding-agent/src/cli.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/cli.ts), [`packages/coding-agent/src/main.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/main.ts) |
| Session lifecycle | [`packages/coding-agent/src/core/agent-session.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/core/agent-session.ts) |
| Runtime assembly | [`packages/coding-agent/src/core/agent-session-runtime.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/core/agent-session-runtime.ts), [`packages/coding-agent/src/core/agent-session-services.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/core/agent-session-services.ts) |
| Session storage | [`packages/coding-agent/src/core/session-manager.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/core/session-manager.ts) |
| Settings and config | [`packages/coding-agent/src/core/settings-manager.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/core/settings-manager.ts), [`packages/coding-agent/src/config.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/config.ts) |
| Event bus | [`packages/coding-agent/src/core/event-bus.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/core/event-bus.ts) |
| Tool execution | [`packages/coding-agent/src/core/tools/`](https://github.com/skaft-software/hamr/tree/main/packages/coding-agent/src/core/tools), [`packages/agent/src/harness/`](https://github.com/skaft-software/hamr/tree/main/packages/agent/src/harness) |
| Providers and model metadata | [`packages/coding-agent/src/core/model-registry.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/core/model-registry.ts), [`packages/ai/src/`](https://github.com/skaft-software/hamr/tree/main/packages/ai/src) |
| Parsing and repair | [`packages/coding-agent/src/hamr/providers/parsers/`](https://github.com/skaft-software/hamr/tree/main/packages/coding-agent/src/hamr/providers/parsers), [`packages/coding-agent/src/hamr/providers/repair.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/hamr/providers/repair.ts), [`packages/coding-agent/src/hamr/providers/tool-calls.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/hamr/providers/tool-calls.ts) |
| Memory and handoff | [`packages/coding-agent/src/hamr/memory.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/hamr/memory.ts), [`packages/coding-agent/src/hamr/memory/HolographicMemory.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/hamr/memory/HolographicMemory.ts), [`packages/coding-agent/src/hamr/handoff/HandoffManager.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/hamr/handoff/HandoffManager.ts) |
| Compaction | [`packages/coding-agent/src/core/compaction/`](https://github.com/skaft-software/hamr/tree/main/packages/coding-agent/src/core/compaction) |
| Extensions | [`packages/coding-agent/src/core/extensions/`](https://github.com/skaft-software/hamr/tree/main/packages/coding-agent/src/core/extensions), [`packages/coding-agent/src/hamr/extensions/`](https://github.com/skaft-software/hamr/tree/main/packages/coding-agent/src/hamr/extensions) |
| Interactive UI | [`packages/coding-agent/src/modes/interactive/`](https://github.com/skaft-software/hamr/tree/main/packages/coding-agent/src/modes/interactive) |

## Runtime Notes

- `packages/coding-agent` owns the CLI, session lifecycle, settings, extensions, and persistence.
- `packages/agent` provides lower-level execution harness primitives.
- `packages/ai` owns provider adapters, streaming, and model metadata.
- The current layout is intentionally flexible. Expect module boundaries to change as the runtime is refactored.

## Turn Flow

1. The CLI parses args in [`packages/coding-agent/src/cli.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/cli.ts) and hands off to the main runtime in [`packages/coding-agent/src/main.ts`](https://github.com/skaft-software/hamr/blob/main/packages/coding-agent/src/main.ts).
2. `AgentSessionRuntime` assembles cwd-bound services, session state, and trust context.
3. `AgentSession` runs the turn loop, streams provider output, executes tools, and persists session entries.
4. The event bus and extension runner fan out lifecycle events to the UI and plugins.
5. Compaction, memory, and session storage kick in as context fills up or sessions branch.

## Working Areas

### Session and storage

`SessionManager` owns session files and tree structure. `AgentSession` uses it for branching, replay, export, and persistence.

### Tool execution

Tool execution is split between the Hamr-specific tool registry and the lower-level harness in `packages/agent`. This split is still being reshaped, so treat it as implementation detail rather than design law.

### Providers and parsing

Provider adapters live in `packages/ai`. Hamr-specific parsing and repair live under `packages/coding-agent/src/hamr/providers/`.

### Memory and compaction

SQLite-backed memory and compaction are active, but the surrounding orchestration is still in motion.

## Historical Note

Legacy Pi-era docs and design notes are kept in `packages/coding-agent/docs/` and the repo history. They are useful for attribution and context, but this page reflects the current Hamr runtime.
