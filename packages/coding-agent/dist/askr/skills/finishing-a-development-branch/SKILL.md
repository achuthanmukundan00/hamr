---
name: finishing-a-development-branch
description: Use when implementation is complete, all tests pass, and you need to decide how to integrate the work - guides completion of development work by presenting structured options for merge, PR, or cleanup
---

# Finish Branch

Announce: `I'm using the finishing-a-development-branch skill to complete this work.`

Flow: verify tests, detect env, present menu, execute choice, clean only when owned.

## 1. Verify

Run full project verification. If failing, show failures and stop. No merge/PR menu until green.

## 2. Detect

```bash
GIT_DIR=$(cd "$(git rev-parse --git-dir)" 2>/dev/null && pwd -P)
GIT_COMMON=$(cd "$(git rev-parse --git-common-dir)" 2>/dev/null && pwd -P)
git branch --show-current
```

Normal repo: `GIT_DIR == GIT_COMMON`.
Worktree: different.
Detached HEAD: no branch; externally managed.

Find base: `git merge-base HEAD main` or `master`; ask if uncertain.

## 3. Menu

Normal repo or named worktree:

```text
Implementation complete. What would you like to do?

1. Merge back to <base-branch> locally
2. Push and create a Pull Request
3. Keep the branch as-is (I'll handle it later)
4. Discard this work

Which option?
```

Detached:

```text
Implementation complete. You're on a detached HEAD (externally managed workspace).

1. Push as new branch and create a Pull Request
2. Keep as-is (I'll handle it later)
3. Discard this work

Which option?
```

No extra explanation.

## 4. Execute

Merge local:
1. cd main repo root, checkout base, pull, merge feature.
2. Verify tests on merged result.
3. If ok, cleanup owned worktree, delete branch.

PR:
1. `git push -u origin <branch>`.
2. Create PR.
3. Preserve worktree for PR iteration.

Keep:
Report branch/path. Preserve.

Discard:
Ask exact confirmation:

```text
This will permanently delete:
- Branch <name>
- All commits: <commit-list>
- Worktree at <path>

Type 'discard' to confirm.
```

Only after exact `discard`: cd main root, cleanup owned worktree, force-delete branch.

## Cleanup

Only options merge/discard. Never cleanup PR/keep.

If normal repo: no worktree cleanup.
If worktree path under `.worktrees/` or `worktrees/`: we own it; `git worktree remove "$WORKTREE_PATH"` from main root, then `git worktree prune`.
Otherwise harness owns it; do not remove. Use platform exit tool if available.

## Never

- offer menu before tests pass
- ask open-ended "what next?"
- remove worktree for PR/keep
- delete branch before removing worktree
- run `git worktree remove` from inside that worktree
- discard without exact confirmation
