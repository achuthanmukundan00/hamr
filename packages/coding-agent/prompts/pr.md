---
description: Turn current worktree changes into a clean pull request
argument-hint: "[base-branch]"
---
1. Run the repo verification commands
2. Inspect the diff with `git diff --stat origin/main` and `git log origin/main..HEAD --oneline`
3. Summarize what changed and why
4. Build a PR body with verification evidence
5. Create the PR against the requested base branch

Call out any breaking changes, unrelated edits, or missing verification before creating the PR.
