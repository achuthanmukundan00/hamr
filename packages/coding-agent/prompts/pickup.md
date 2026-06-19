---
description: Find open issues, group related ones, and start work in an isolated worktree
argument-hint: "[milestone-filter] [label-filter]"
---
Read the project context first.

## Find and pick up work

1. List open issues: `gh issue list --state open --limit 200 --json number,title,labels,milestone,assignees`
2. Group issues by shared files, shared specs, milestones, and area labels
3. Check for blockers and dependency chains
4. Present 3-5 workable clusters in a compact table
5. Ask which cluster to pick up
6. Create a worktree under `../.worktrees/<cluster-name>`
7. Build the repo and start the first unblocked issue

Prefer small clusters over big mixes. If two issues touch the same file, do them in sequence.
