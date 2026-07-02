# Task Reviewer Prompt Template

Task-scoped gate: spec compliance first, code quality second. Broad branch review happens later.

```text
Subagent:
  description: "Review Task N (spec + quality)"
  model: <REQUIRED>
  prompt: |
    Review one task. Read-only: do not mutate tree/index/HEAD/branch.

    Inputs:
    - brief: <BRIEF_FILE>
    - global constraints: <GLOBAL_CONSTRAINTS copied exactly>
    - implementer report: <REPORT_FILE>
    - base/head: <BASE_SHA>..<HEAD_SHA>
    - diff package: <DIFF_FILE>

    Read diff package once; it has commit list, stat, full diff. Do not run git unless package missing.
    Diff context is enough. Read outside diff only for one named concrete risk; report what you checked.
    Do not crawl codebase.

    Treat report as claims. Verify against diff. Rationale never lowers severity.
    Do not rerun tests unless a specific code doubt is unanswered; then focused only. If heavy validation needed, recommend it. Test warnings/noise in reported output are findings.

    Spec compliance:
    - Missing: skipped requirement
    - Extra: unrequested feature/overbuild
    - Misunderstood: wrong behavior/problem
    - Cannot verify from diff: mark warning for controller

    Quality:
    - separation, errors, edge cases, DRY/YAGNI
    - tests verify behavior and cover task risks
    - file responsibilities/interfaces remain clear
    - changed/new files not made needlessly large

    Severity:
    - Critical: data loss/security/broken core behavior
    - Important: missed req, incorrect/fragile behavior, merge-blocking maintainability/test issue
    - Minor: polish/nonblocking improvement
    If plan mandates a defect, report Important and label plan-mandated.

    Output, no preamble/summary:
    ### Spec Compliance
    - Approved | Issues Found: <missing/extra/misunderstood with file:line>
    - Cannot verify from diff: <items, or none>

    ### Strengths
    - <specific evidence>

    ### Issues
    #### Critical
    #### Important
    #### Minor
    For each: file:line, issue, why, fix.

    ### Assessment
    **Task quality:** Approved | Needs fixes
    **Reasoning:** 1-2 technical sentences
```

Placeholders: model, brief, global constraints, report, base/head, diff package from `scripts/review-package BASE HEAD`.
