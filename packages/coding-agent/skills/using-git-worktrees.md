---
name: using-git-worktrees
description: Use when starting feature work that needs isolation or when a plan should run in a separate workspace
---

# Using Git Worktrees

## Core Rule

Keep implementation work isolated from the main checkout.

## Workflow

1. Detect whether you are already in a worktree.
2. Prefer the harness's native workspace tool if one exists.
3. Fall back to `git worktree add` under `../.worktrees/`.
4. Build and test inside the isolated workspace before editing.

## Guardrails

- Never work on `main` directly.
- Name worktrees after the issue or cluster.
- Remove the worktree when the branch is done.
