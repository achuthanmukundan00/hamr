---
name: writing-plans
description: Use when you have a spec or requirements for a multi-step task, before touching code
---

# Write Plans

Announce: `I'm using the writing-plans skill to create the implementation plan.`

Goal: plan for skilled engineer with zero repo context and weak test judgment. Give exact files, code, commands, expected output. DRY, YAGNI, TDD, frequent commits.

Save: `docs/askr/plans/YYYY-MM-DD-<feature>.md` unless user says otherwise.

If spec covers independent subsystems, stop and split into one plan per subsystem.

## Before Tasks

Map file structure first:
- create/modify/test files
- each file responsibility
- interfaces between units
- existing patterns to follow
- focused splits only when current touched file is too large/confused

Task size: one reviewable, independently testable deliverable. Fold setup/docs into task that needs them. Split only where reviewer could reject one task and approve neighbor.

## Required Header

```markdown
# [Feature Name] Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use askr:subagent-driven-development (recommended) or askr:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** [one sentence]

**Architecture:** [2-3 sentences]

**Tech Stack:** [key tech/libs]

## Global Constraints

[project-wide requirements copied verbatim from spec: versions, dependency limits, names, copy, platforms]

---
```

## Task Template

Each task:

````markdown
### Task N: [Name]

**Files:**
- Create: `path`
- Modify: `path:line`
- Test: `path`

**Interfaces:**
- Consumes: exact names/signatures from earlier tasks
- Produces: exact names/signatures later tasks need

- [ ] Step 1: Write failing test
```lang
actual test code
```

- [ ] Step 2: Run test; expect fail
Run: `exact command`
Expected: FAIL because ...

- [ ] Step 3: Minimal implementation
```lang
actual code
```

- [ ] Step 4: Run test; expect pass
Run: `exact command`
Expected: PASS

- [ ] Step 5: Commit
```bash
git add ...
git commit -m "..."
```
````

## No Placeholders

Never write: TBD, TODO, implement later, add validation, handle edge cases, write tests, similar to Task N, appropriate error handling, undefined future type/function, code step without code, command without expected result.

Exact paths, exact commands, exact expected output.

## Self-Review

Before handoff:
1. Spec coverage: every requirement maps to task.
2. Placeholder scan: fix banned phrases.
3. Type/name consistency across tasks.
4. Scope: plan produces working testable software.

Fix inline until clean.

## Handoff

Say:

`Plan complete and saved to <path>. Two execution options:
1. Subagent-Driven (recommended) - fresh subagent per task, review between tasks.
2. Inline Execution - execute in this session with checkpoints.
Which approach?`

If 1: invoke `subagent-driven-development`. If 2: invoke `executing-plans`.
