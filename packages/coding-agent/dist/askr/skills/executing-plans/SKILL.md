---
name: executing-plans
description: Use when you have a written implementation plan to execute in a separate session with review checkpoints
---

# Execute Plan

Use when no subagent workflow will run in this session. If subagents exist, prefer `subagent-driven-development`.

Announce: `I'm using the executing-plans skill to implement this plan.`

## Flow

1. Read plan.
2. Review for contradictions, missing info, unsafe branch (`main`/`master`), unclear steps.
3. If concern blocks work, ask human once with specifics.
4. Create todos for plan tasks.
5. For each task: mark in progress, follow steps exactly, run specified verification, commit if plan says, mark done.
6. After all verified, invoke `finishing-a-development-branch`.

## Stop

Stop and ask when dependency missing, instruction unclear, tests repeatedly fail, plan has critical gap, or implementation would violate user instructions.

Do not guess through blockers. Do not skip verification. Reference required skills named by plan.

Workflow deps: `using-git-worktrees`, `writing-plans`, `finishing-a-development-branch`.
