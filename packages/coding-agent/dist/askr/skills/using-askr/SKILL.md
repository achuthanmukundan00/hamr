---
name: using-askr
description: Use when a task could be served by an askr skill, to find and apply the right one. Not for greetings or trivial replies.
---

# Using Askr

Askr skills are bundled with Hamr. When a skill clearly fits the task, use it before improvising.

## Core Rule

If a bundled skill clearly fits the task, use it before improvising.

## What To Do

1. Read only the context the task actually needs — don't sweep the repo for small or open-ended asks.
2. Check the bundled skills for a direct match.
3. Prefer the smallest workflow that still gives a correct result.
4. Ask one clear question at a time when requirements are unclear.
5. Keep work verifiable and file-scoped.

## Defaults

- Use `frontend-design` when designing a TUI, web app, or other UI-heavy system.
- Design before code when the request is ambiguous.
- Verify before declaring completion.
- Review diffs before merging.

## Bundled Skills

| Skill | When |
|-------|------|
| `frontend-design` | Designing UI/TUI/web apps |
| `subagent-driven-development` | Executing implementation plans with independent tasks |
| `writing-plans` | Specs or requirements for multi-step tasks, before touching code |
| `executing-plans` | Written plan to execute in a separate session |
| `test-driven-development` | Implementing features or bugfixes |
| `systematic-debugging` | Any bug, test failure, or unexpected behavior |
| `requesting-code-review` | Completing tasks, major features, before merging |
| `receiving-code-review` | Receiving code review feedback |
| `verification-before-completion` | About to claim work is complete |
| `dispatching-parallel-agents` | 2+ independent tasks |
| `finishing-a-development-branch` | Implementation done, deciding how to integrate |
| `using-git-worktrees` | Feature work needing isolation |
| `writing-skills` | Creating or editing skills |
