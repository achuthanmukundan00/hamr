---
description: Pick up a GitHub issue, isolate the work, implement it, verify it, and open a PR
argument-hint: "<issue-number> [scope]"
---
Before doing anything else, read the project context (`AGENTS.md`, `README.md`, `docs/`, and any referenced specs).

## Pick up issue #$1

1. Fetch the issue: `gh issue view $1 --json number,title,body,labels,assignees,comments,milestone`
2. Create or reuse an isolated worktree under `../.worktrees/issue-$1`
3. Install and build the repo
4. Read the files the issue touches and nearby tests
5. Write a small plan as a comment on the issue
6. Implement in small steps with verification after each step
7. Run the full verification suite, then open a PR

If `$2` is provided, keep the work inside that scope.

Do not work on `main` directly. Do not skip verification. Do not change unrelated files.
