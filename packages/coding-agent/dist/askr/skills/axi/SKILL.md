---
name: axi
description: Agent eXperience Interface (AXI) — ergonomic standards for building and running CLI tools that agents use
---

# Agent eXperience Interface (AXI)

AXI makes CLIs cheap and reliable for agents: low-token stdout, no prompts, deterministic exits, and self-correcting errors.

## Output Contract

- **Stdout is data.** Print TOON for normal output, errors, confirmations, and help hints. Keep progress/debug logs on stderr.
- **Use TOON at the boundary.** Keep internals JSON if useful, but emit compact rows:
  ```
  tasks[2]{id,title,status}:
    1,Fix auth bug,open
    2,Add pagination,closed
  ```
- **Default lists are tiny.** Usually 3-4 fields: id/name/status plus one next-action field. Offer `--fields` for more.
- **Long content is previewed, not omitted.** Truncate large bodies, include total size, and show the exact `--full` command only when truncated.
- **Aggregate up front.** Include total counts/status summaries so agents do not paginate just to know scope.
- **Empty is explicit.** `tasks: 0 open tasks found` beats blank output.

## Command Behavior

- **No args = home view, not a manual.** Show current live state, executable path, one-line purpose, and 2-4 relevant next commands.
- **Contextual disclosure.** Help hints must be complete commands that fit the current output; avoid generic “see docs”.
- **No interactive prompts.** Missing input is a structured usage error with exit code 2.
- **Unknown input fails loud.** Reject unknown flags/args before side effects; include valid flags or the corrected replacement.
- **Idempotent mutations.** Desired state already exists => exit 0 and say no-op.
- **Exit codes:** 0 success/no-op/gate, 1 runtime failure, 2 bad usage.

## Agent Integrations

- Prefer explicit `setup hooks`/plugin install commands; ordinary commands must not mutate agent config.
- Hook context must be directory-scoped, token-budget-aware, and content-first.
- Support multiple harnesses when practical; do not hard-code one agent if the UX is portable.
- If shipping a skill too, generate it from the same CLI guidance/home text, strip live state, and make examples non-interactive (`npx -y tool ...`).

## Review Checklist

- Can an agent complete common tasks in one call from the home view?
- Are all errors actionable from stdout alone?
- Are stderr/progress logs impossible to confuse with data?
- Are lists compact by default but lossless via `view`, `--fields`, or `--full`?
- Do repeated mutations and setup commands behave as safe no-ops?
