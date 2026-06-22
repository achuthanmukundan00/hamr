---
name: using-git-worktrees
description: Use when starting feature work that needs isolation from current workspace or before executing implementation plans - ensures an isolated workspace exists via native tools or git worktree fallback
---

# Isolate Work

Announce: `I'm using the using-git-worktrees skill to set up an isolated workspace.`

Principle: detect existing isolation, prefer native harness worktree, fallback to git. Do not fight harness.

## 0. Detect

```bash
GIT_DIR=$(cd "$(git rev-parse --git-dir)" 2>/dev/null && pwd -P)
GIT_COMMON=$(cd "$(git rev-parse --git-common-dir)" 2>/dev/null && pwd -P)
BRANCH=$(git branch --show-current)
git rev-parse --show-superproject-working-tree 2>/dev/null
```

If `GIT_DIR != GIT_COMMON` and not submodule: already isolated. Report path/branch; skip creation.

If normal checkout: honor instruction-file preference. If none, ask: `Would you like me to set up an isolated worktree? It protects your current branch from changes.`

If declined, work in place.

## 1. Create

If platform has native worktree/isolation tool, use it. Only use git fallback if no native tool.

Git fallback:
1. Directory priority: explicit user preference > existing `.worktrees/` > existing `worktrees/` > `.worktrees/`.
2. For project-local dir, verify ignored:
   ```bash
   git check-ignore -q .worktrees 2>/dev/null || git check-ignore -q worktrees 2>/dev/null
   ```
   If not ignored, add to `.gitignore` and commit first.
3. Create:
   ```bash
   git worktree add "$LOCATION/$BRANCH_NAME" -b "$BRANCH_NAME"
   cd "$LOCATION/$BRANCH_NAME"
   ```

If sandbox blocks creation, tell user and work in current directory.

## 2. Setup

Run detected setup only:
- `package.json` -> `npm install`
- `Cargo.toml` -> `cargo build`
- `requirements.txt` -> `pip install -r requirements.txt`
- `pyproject.toml` -> project installer
- `go.mod` -> `go mod download`

## 3. Baseline

Run project tests (`npm test`, `cargo test`, `pytest`, `go test ./...`, etc.).

If fail: report and ask whether to proceed or debug.
If pass: report:

```text
Worktree ready at <path>
Tests passing (<N> tests, 0 failures)
Ready to implement <feature>
```

## Never

- create nested worktree
- use git fallback when native tool exists
- skip ignore check for project-local worktree dir
- proceed over failing baseline without asking
- create worktree on `main`/`master` work without consent
