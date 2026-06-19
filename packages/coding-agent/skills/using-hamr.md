---
name: using-hamr
description: Use when starting any Hamr conversation or before any action; establishes the bundled skill discipline
---

# Using Hamr

## Core Rule

If a bundled skill clearly fits the task, use it before improvising.

## What To Do

1. Read the current project context first.
2. Check the bundled skills for a direct match.
3. Prefer the smallest workflow that still gives a correct result.
4. Ask one clear question at a time when requirements are unclear.
5. Keep work verifiable and file-scoped.

## Defaults

- Use `design-brainstorming` when designing a TUI, web app, or other UI-heavy system.
- Design before code when the request is ambiguous.
- Verify before declaring completion.
- Review diffs before merging.

## Askr Equivalents

Use the nearest askr skill when you need the old prompt-template behavior.

| Old workflow | Askr equivalent |
| --- | --- |
| `devour` | `design-brainstorming` |
| `issue` | `subagent-driven-development` |
| `pr` | `requesting-code-review` |
| `review` | `receiving-code-review` |

`writing-plans` and `using-git-worktrees` are intentionally not bundled into Hamr right now.
