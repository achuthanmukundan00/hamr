---
name: requesting-code-review
description: Use when completing tasks, implementing major features, or before merging to verify work meets requirements
---

# Request Review

Dispatch reviewer with work product, not session history.

Mandatory after each SDD task, major feature, before merge. Useful when stuck or after complex bug fix.

## How

1. Pick base/head SHAs.
2. Fill [code-reviewer.md](code-reviewer.md):
   - `{DESCRIPTION}`: what changed
   - `{PLAN_OR_REQUIREMENTS}`: what it must satisfy
   - `{BASE_SHA}`, `{HEAD_SHA}`
3. Dispatch reviewer.
4. Act:
   - Critical: fix now.
   - Important: fix before proceeding.
   - Minor: record/triage.
   - Wrong finding: push back with code/tests.

Never skip because "simple." Never proceed with open Critical/Important. Review early so defects do not compound.
