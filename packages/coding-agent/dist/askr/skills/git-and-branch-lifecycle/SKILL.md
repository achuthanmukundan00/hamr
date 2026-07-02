---
name: git-and-branch-lifecycle
description: Use when setting up a feature branch, isolating work, requesting/receiving code reviews, or merging a finished branch
---

# Git and Branch Lifecycle

This skill governs workspace setup, code review interaction, and branch integration.

## 1. Setup and Isolation

Protect your primary branches by working in isolated workspaces.

### Worktree Setup Flow
1. **Detect Existing Isolation:** Run `git status` or inspect directories. If already in a worktree or detached HEAD, skip creation.
2. **Create Worktree (Fallback):** If isolation is desired:
   ```bash
   git worktree add ".worktrees/feature-branch" -b "feature-branch"
   cd .worktrees/feature-branch
   ```
3. **Setup and Baseline:** Run package installations (e.g., `npm install`, `cargo build`) and project tests. If baseline tests fail, report the failures to the user and ask how to proceed. Do not fix unrelated baseline issues without explicit user direction.

---

## 2. Review Lifecycle

### Requesting Reviews
When completing a major feature, dispatch a reviewer subagent with the changes:
1. Generate a diff package: `git diff <base-branch>..HEAD`.
2. Fill out the reviewer prompt using the [code-reviewer.md](code-reviewer.md) template.
3. Review findings:
   * **Critical & Important:** Must be fixed immediately.
   * **Minor:** Record in ledger for final polish.

### Receiving Reviews
* **Evidence Over Agreement:** Do not agree blindly. Check suggestion validity against the code.
* **Push Back with Code:** If a suggestion breaks compatibility or violates YAGNI, respond with reasoning backed by code references or tests.
* **Concise Replies:** Keep review replies functional and direct (e.g., `Fixed: <commit-sha>` or `Reason for pushback: <rationale>`).

### Inline Review Fallback

If a dedicated reviewer subagent is not available (e.g., due to model limitations or context constraints), the agent should self-review the diff against these dimensions:

- **Correctness:** Does the code do what it intends? Are there logic errors, off-by-one bugs, or race conditions?
- **Test Coverage:** Are new behaviors covered by tests? Do existing tests still pass?
- **Edge Cases:** What happens with empty input, null/undefined, boundary values, or network failures?
- **Naming:** Do function/variable/type names communicate intent clearly and consistently with the codebase?
- **Error Handling:** Are errors surfaced with meaningful messages? Are unexpected failures caught gracefully?
- **API Compatibility:** Does the change break existing callers? Are public signatures backward-compatible?

Fix any issues found before marking the task complete.

---

## 3. Finishing a Branch

Once all work is complete, verify tests pass locally, then present the integration menu:

### Integration Options
```text
Implementation complete. What would you like to do?

1. Merge back to <base-branch> locally
2. Push and create a Pull Request
3. Keep the branch as-is
4. Discard this work
```

### Execution Rules
* **Merge Local:** Checkout the base branch, merge, verify tests, and clean up the worktree.
* **PR:** Push the branch and present the PR link. Keep the worktree intact for feedback iterations.
* **Cleanup:** When merging or discarding, remove the worktree:
  ```bash
  git worktree remove <path>
  git worktree prune
  ```
