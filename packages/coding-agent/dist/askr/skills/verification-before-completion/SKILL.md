---
name: verification-before-completion
description: Use when about to claim work is complete, fixed, or passing, before committing or creating PRs - requires running verification commands and confirming output before making any success claims; evidence before assertions always
---

# Verify Before Claim

Iron law:

```text
NO COMPLETION CLAIM WITHOUT FRESH EVIDENCE
```

If you did not run the proof command in this message, do not claim it passes.

## Gate

Before success/completion/correctness claim:
1. Identify command/evidence that proves claim.
2. Run full fresh command/check.
3. Read output and exit code.
4. If not proven, state actual status.
5. If proven, claim with evidence.

## Claims Need

| Claim | Evidence |
|---|---|
| tests pass | test output, 0 failures |
| lint clean | linter output, 0 errors |
| build succeeds | build command exit 0 |
| bug fixed | original repro/test now passes |
| regression test works | red-green verified |
| agent completed | diff inspected + verification |
| reqs met | checklist against spec/plan |

## Red Flags

"should", "probably", "seems", "done", "fixed", "perfect", "looks good", tired, partial check, trusting subagent, commit/PR without fresh verification.

All mean stop and verify.

## Patterns

- Tests: run command, see pass count, then say pass.
- Regression: test passes, revert fix to see fail, restore fix to see pass.
- Build: run build; lint is not compiler.
- Requirements: re-read plan/spec; check line by line.
- Delegation: subagent report is not evidence; inspect diff and run checks.

Bottom line: run, read, then claim.
