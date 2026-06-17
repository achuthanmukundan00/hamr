# Architecture

Hamr is a TypeScript, Bun-native CLI and TUI for local-first coding agents.

## Runtime Stack

- **Runtime**: Bun (required for the TUI's Rust native addon via `sexy-tui-rs`)
- **Language**: TypeScript (strict mode)
- **Build**: `tsgo` (TypeScript compiler)
- **Lint/Format**: Biome
- **Test**: Jest with `@swc/jest` transformer
- **TUI**: `sexy-tui-rs` (Rust native addon providing Editor, TUI, Markdown, select lists)
- **Docs**: VitePress

## Module Map

```
CLI (cli.ts)
  ├── commands/          # Command implementations
  │   ├── chat.ts        # Interactive TUI session
  │   ├── run.ts         # Single-turn agent execution
  │   ├── ask.ts         # Read-only codebase queries
  │   ├── inspect.ts     # Session browser
  │   ├── config.ts      # Config management
  │   └── doctor.ts      # Provider/model diagnostics
  ├── session/           # Session lifecycle
  │   ├── Session.ts     # Core turn loop, tool execution, recovery
  │   ├── SessionFactory.ts
  │   ├── message-assembly.ts  # Budget guard, compaction
  │   ├── formatting.ts        # Message construction, safety checks
  │   ├── tool-definitions.ts  # Model-facing tool schemas + system prompt
  │   └── verification-contracts.ts
  ├── agent/             # Agent control
  │   ├── run-task.ts    # Orchestration, verification, repair
  │   ├── context-budget.ts    # Token estimation, compaction
  │   ├── dispatch-intent.ts   # Fast-path dispatch classification
  │   ├── task-policy.ts       # Run modes, tool gating
  │   ├── safety.ts      # Checkpoints, dirty tree detection
  │   └── verification.ts
  ├── llm/               # LLM interface
  │   ├── client.ts      # OpenAI-compatible HTTP client + streaming
  │   ├── provider-factory.ts
  │   ├── parsers/       # 12 tool-call parsers
  │   ├── repair/        # JSON/XML repair, reasoning sanitizer
  │   └── tool-calls.ts  # Tool call registry
  ├── tui/               # Terminal UI
  │   ├── hamr-tui.ts    # Main TUI orchestration
  │   ├── components/    # EventFeed, StatusBar, AgentDashboard
  │   ├── theme/         # 9 color themes
  │   └── semantic-events.ts  # AgentEvent → UI card classifier
  ├── actions/           # Tool execution
  │   ├── ActionExecutor.ts
  │   └── handlers/      # bash, read, edit, write, memory, image
  ├── tools/             # Tool infrastructure
  │   ├── registry.ts
  │   ├── policy.ts      # File safety, path validation
  │   └── ledger.ts      # Inspection tracking
  ├── orchestration/     # Sub-agent system
  │   ├── OrchestrationManager.ts
  │   ├── plan-parser.ts
  │   ├── conflict-detector.ts
  │   └── dependency-resolver.ts
  └── sdk/               # Embeddable SDK
      └── HamrRuntime.ts
```

## Key Design Decisions

### Local-First, Not Cloud-First

Hamr assumes local models will emit malformed outputs. Every layer has defense-in-depth:
- 12 tool-call parsers with auto-repair
- Reasoning/thinking tag sanitization
- Mixed-output detection and stripping
- Truncation salvage (complete XML blocks recovered from cut-off responses)
- Planning prose detection ("Let me analyze..." without tool calls)

### Context Budget as a Hard Constraint

The context window is tracked and enforced at every model call. Multi-stage deterministic compaction runs before the budget is exceeded. When compaction fails, the session can handoff to a child session with FTS5 memory inheritance.

### Bounded Loop with Progressive Escalation

The agent loop runs up to 64 model steps and 192 tool calls. Progressive guardrails escalate from gentle nudges to hard stops as the model repeats unproductive patterns (re-reading files, planning without acting, running identical commands).

### TUI Event Pipeline

AgentEvents flow through `classifyAgentEvent()` which maps them to semantic UI cards. The EventFeed renders cards with model-family glyphs, color-coded status, Markdown formatting, and background shading. Cards auto-scroll and support mouse/trackpad navigation in alternate screen mode.

## File Size Conventions

- Keep files under ~500 lines where practical
- Extract focused modules rather than extending large files
- Session.ts is the largest file (~2100 lines) — it owns the core turn loop and tool execution
