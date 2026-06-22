---
name: subagent-driven-development
description: Use when executing implementation plans with independent tasks in the current session
---

# Subagent-Driven Development

Fresh implementer per task, task review after each, whole-branch review at end. Controller keeps context small; artifacts move as files.

No check-ins between tasks. Stop only when blocked, ambiguous enough to prevent progress, or all tasks complete. Narrate max one short line between tools.

## Use When

Have implementation plan, tasks mostly independent, and staying in current session. If no plan, brainstorm/write plan. If execution is in separate session, use `executing-plans`.

Required workflow skills: `using-git-worktrees`, `writing-plans`, `requesting-code-review`, `finishing-a-development-branch`. Implementers use `test-driven-development`.

## Start

1. Read plan once.
2. Scan for plan conflicts/global-constraint contradictions. If found, batch question to human before execution.
3. Check ledger:
   ```bash
   cat "$(git rev-parse --git-path sdd)/progress.md"
   ```
   Completed tasks there are done; resume first incomplete. Trust ledger/git after compaction.
4. Create todos.
5. Record branch start/merge base for final review.

## Per Task

1. Run `scripts/task-brief PLAN_FILE N`; use printed brief path.
2. Dispatch implementer with:
   - one line where task fits
   - brief path as exact requirements
   - needed interfaces/decisions from prior tasks only
   - ambiguity resolutions
   - report path (`task-N-report.md`) and contract
   - explicit model
3. Implementer writes full report file; returns only status, commits, one-line tests, concerns.
4. On DONE, run `scripts/review-package BASE HEAD`; use recorded pre-task BASE, never `HEAD~1`.
5. Dispatch task reviewer with brief path, report path, review package path, copied global constraints.
6. If reviewer has Critical/Important/spec fail, dispatch fix subagent with all findings and covering tests. Fix report appends to same report file. Re-review.
7. Resolve any "cannot verify from diff" yourself before marking done.
8. On clean review, append ledger line:
   `Task N: complete (commits <base7>..<head7>, review clean)`.

## Status Handling

- DONE: package diff, review.
- DONE_WITH_CONCERNS: read concerns. Correctness/scope concern must be addressed before review; observation can proceed.
- NEEDS_CONTEXT: supply missing context and re-dispatch.
- BLOCKED: change something: more context, stronger model, smaller task, or ask human if plan wrong. Never retry same prompt/model blindly.

## Reviewer Prompt Rules

Task review is task-scoped. Final review is broad.

- Do not ask open-ended "check all uses" without task-specific reason.
- Do not ask reviewer to rerun same tests already evidenced.
- Do not pre-judge findings (`do not flag`, `Minor at most`, `plan chose`).
- Global constraints block is exact project requirements only; copy exact values/relationships from spec/plan.
- Give diff as file via `review-package`; do not paste diff.
- Prompt one task, not accumulated history.
- Critical/Important get fixed now. Minor goes in ledger for final triage.
- Plan-mandated finding goes to human with finding plus plan text.
- Final review also gets package: `scripts/review-package MERGE_BASE HEAD`.
- Final-review fixes: one fix subagent with full findings list, not one per finding.

## File Handoffs

Never paste large artifacts into dispatch or accept large reports in chat.

- Brief: `scripts/task-brief PLAN_FILE N`.
- Report: implementer writes report file; chat return stays tiny.
- Reviewer inputs: brief, report, review package, global constraints.
- Fixes: append to same report file; return tiny summary.

## Models

Always specify model.

- complete code/transcription or 1-file mechanical: cheapest.
- prose implementer or reviewer floor: mid-tier (turn count matters).
- multi-file integration/debugging: standard.
- architecture/final whole-branch review: most capable.

## Finish

After all tasks:
1. Run final review with whole-branch package and Minor ledger.
2. Fix final findings in one fix wave.
3. Invoke `finishing-a-development-branch`.

## Never

- implement on `main`/`master` without explicit consent
- dispatch multiple implementers in parallel
- skip task review or accept missing spec/quality verdict
- move on with open Critical/Important/spec failures
- make subagent read whole plan instead of brief
- ignore questions/escalations
- let self-review replace independent review
- re-dispatch task marked complete in ledger
