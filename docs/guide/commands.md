# Commands

## `hamr`

With no arguments, Hamr starts `chat` in full-screen TUI mode when attached to a TTY.

```sh
bun run hamr --
```

## `hamr chat`

Interactive agent shell (full-screen TUI by default on TTY):

```sh
bun run hamr -- chat
```

The full-screen TUI uses sexy-tui-rs and runs under Bun. Use `--plain` when you need the non-TUI fallback.
Use `--cmux-mode` when running many parallel TUI sessions and you want lower frame churn.

Plain fallback:

```sh
bun run hamr -- chat --plain
```

Single-turn chat:

```sh
bun run hamr -- chat --message "Explain the test layout."
```

Slash commands:

| Command                        | Behavior                                              |
| ------------------------------ | ----------------------------------------------------- |
| `/help`                        | Show available chat commands                          |
| `/model [<model-id>]`          | Switch AI model or open model picker                  |
| `/theme [<theme-name>]`        | Change color theme or open theme picker               |
| `/settings`                    | Show provider, agent, tool, and verification settings |
| `/tools`                       | Show model-facing tool surface                        |
| `/budget`                      | Show context and loop limits                          |
| `/cost`                        | Show token usage and cost summary                     |
| `/context`                     | Show context budget breakdown                         |
| `/skill <name>`                | Load a skill by name at runtime                       |
| `/skills`                      | List available and loaded skills                      |
| `/status`                      | Show git, budget, checkpoint, and read-state summary  |
| `/new`                         | Clear feed and start fresh                            |
| `/clear`                       | Reset the chat conversation and inspection ledger     |
| `/resume`                      | List or resume past sessions                          |
| `/diff`                        | Show bounded git diff                                 |
| `/verify [quick \| full]`     | Run configured verification command                   |
| `/exit`, `/quit`               | Exit chat                                             |

For large pasted prompts, use terminal bracketed paste. Hamr detects paste boundaries and renders the
paste as an inline attachment chip:

```txt
hamr> take a look at this [pasted: 84 lines, 12.4k chars] and modify the twelfth line
```

The full pasted body is held locally until Enter is pressed, then Hamr submits a single canonical
message that preserves the typed text before and after the paste. Typed slash commands still execute as
commands, but a slash command inside a paste is treated as literal content.

## `hamr ask`

Runs one bounded question or task and exits:

```sh
bun run hamr -- ask --question "Where is provider config normalized?"
```

Output modes:

```sh
bun run hamr -- ask --question "Summarize the CLI" --quiet
bun run hamr -- ask --question "Summarize the CLI" --json
bun run hamr -- ask --question "Summarize the CLI" --json --debug
```

`--quiet` and `--json` cannot be combined. `--quiet` and `--debug` cannot be combined.

## `hamr run`

Runs one bounded edit-capable agent task:

```sh
bun run hamr -- run --task "Fix the failing auth test"
```

Task modes constrain the tool surface:

```sh
bun run hamr -- run --mode read-only --task "Inspect the registry and summarize safe improvements"
bun run hamr -- run --mode patch --task "Make one docs-only wording improvement in README.md"
bun run hamr -- run --mode verify --task "Inspect the patch and report whether verification is safe"
bun run hamr -- run --mode docs --task "Update docs/guide/commands.md with one small wording fix"
```

Replacement edits print a patch preview before writing. Because `run` is non-interactive, previewed replacement
edits are rejected by default. Pass `--yes` to accept previewed replacement edits during that run:

```sh
bun run hamr -- run --task "Fix the failing auth test" --yes
bun run hamr -- run --task "Fix the failing auth test" --yes --verification-profile full --repair-attempts 1
```

Plan files are not implemented yet:

```sh
bun run hamr -- run --plan plan.md
```

Verification profiles and budget control:

```sh
bun run hamr -- run --task "Fix the failing auth test" --verification-profile quick
bun run hamr -- run --task "Fix the failing auth test" --verification-profile full
bun run hamr -- run --task "Fix the failing auth test" --budget 0.50
bun run hamr -- run --task "Fix the failing auth test" --strategy aggressive
bun run hamr -- run --task "Fix the failing auth test" --no-skills
```

Verification contract levels (gate model behavior):

```sh
bun run hamr -- run --task "Fix the failing auth test" --verify none
bun run hamr -- run --task "Fix the failing auth test" --verify tests-passing
```

Run control-surface TUI (stable frame, no log spam):

```sh
bun run hamr -- run --task "Fix the failing auth test" --tui
bun run hamr -- run --task "Fix the failing auth test" --tui --cmux-mode
```

The TUI is an opt-in MVP that shows a fixed-frame control surface during `hamr run`:

- Phase machine (idle → thinking → tool_execution → verifying → completed/blocked/error)
- Severity ladder (S0–S3) and risk line
- Compact timeline of recent events
- Change file list with overflow compression
- Verification lifecycle counts (planned, running, passed, failed, skipped)
- 9×9 AI core overlay indicating internal state
- SIGWINCH resize repaint

Current run-TUI limitations:

- `hamr run --tui` remains the non-interactive run surface
- Fixed layout; no scrolling panes or log streaming
- No interactive controls beyond `q` to quit
- Verification status is derived from lifecycle events emitted by the runtime (not summary text parsing)
- Small terminals (< 40 cols, < 18 rows) show a minimal warning

## `hamr inspect`

Inspects project metadata, context state, skills, and run metrics:

```sh
bun run hamr -- inspect
bun run hamr -- inspect --json
bun run hamr -- inspect --profile
bun run hamr -- inspect --brief
bun run hamr -- inspect --section git --section packageManager
bun run hamr -- inspect --docs
bun run hamr -- inspect --doc specs/PRD.md
bun run hamr -- inspect --search-docs "relay"
bun run hamr -- inspect --docs-impact
```

Context ledger inspection:

```sh
bun run hamr -- inspect --ledger       # Context state from last chat session
bun run hamr -- inspect --context      # Expanded context state (JSON)
bun run hamr -- inspect --budget       # Budget configuration
```

Skills inspection:

```sh
bun run hamr -- inspect --skills       # List all discovered skills
bun run hamr -- inspect --skill <name> # Show instructions for a specific skill
```

Run metrics from the event store:

```sh
bun run hamr -- inspect --metrics                 # Recent sessions table
bun run hamr -- inspect --metrics --json          # Machine-readable JSON
bun run hamr -- inspect --metrics --session <id>  # Timeline for a session
bun run hamr -- inspect --metrics --stats         # Aggregate stats (30 days)
```

`--docs` lists the bounded local docs/spec files Hamr recognizes. `--doc <path>` reads one recognized
docs/spec file with line numbers and the same secret redaction used by the local docs provider.
`--search-docs <query>` performs deterministic bounded text search across recognized docs.
`--docs-impact` reports when behavior-facing source changes likely need docs updates.

## `hamr config`

```sh
bun run hamr -- config init
bun run hamr -- config show
bun run hamr -- config get provider.model
```

## `hamr doctor`

Quick local checks:

```sh
bun run hamr -- doctor
```

Full provider checks:

```sh
bun run hamr -- doctor --full
```

## Typical Workflow

<MeasuredTerminalBlock
  title="hamr session"
  :lines="[
    'hamr chat',
    '/status',
    '  core: qwen-coder',
    '  provider: relay (local)',
    '  ctx: 0 / 32768',
    '',
    '> fix the type error in src/config.ts',
    '',
    '  reasoning: type mismatch...',
    '  read src/config.ts',
    '  edit src/config.ts',
    '  test bun run test',
    '  result: verification passed',
    '',
    '/status',
    '  state: succeeded',
  ]"
  :dimLines="[4, 5, 6, 7]"
  ariaLabel="Example Hamr session showing status, a coding request, tool calls, and completion"
/>
