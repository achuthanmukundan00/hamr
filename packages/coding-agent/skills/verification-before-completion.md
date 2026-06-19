---
name: verification-before-completion
description: Use when about to claim work is complete, fixed, or passing before committing or creating PRs
---

# Verification Before Completion

## Core Rule

Do not claim success without fresh verification evidence.

## Workflow

1. Identify the command that proves the claim.
2. Run the full command now.
3. Read the output and the exit code.
4. State the actual result, not the hoped-for one.

## Guardrails

- "Should work" is not evidence.
- Do not trust prior runs, summaries, or agent claims.
- Verify the exact behavior that changed.
