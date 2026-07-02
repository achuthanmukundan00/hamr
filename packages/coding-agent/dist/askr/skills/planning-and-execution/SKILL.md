---
name: planning-and-execution
description: Use when you have a multi-step task, feature spec, or requirements to plan and implement
---

# Planning and Execution Lifecycle

This skill covers the unified process of analyzing requirements, drafting an implementation plan, and executing that plan using inline checkpoints, subagents, or parallel agents.

## 1. Planning Phase (Spec to Tasks)

Before modifying code, write an implementation plan to `docs/askr/plans/YYYY-MM-DD-<feature>.md`.

### Principles for LLMs
* **No Source Code in Plans (Avoid Double Generation):** Do not write full implementation code or full test code in the plan. Instead, specify file paths, class/interface signatures, and expected inputs/outputs. Let the implementer generate the code when it has direct access to compiling and testing tools.
* **Granular Tasks:** Divide the feature into reviewable, independently testable tasks. A task should represent a single commit.

### Plan Template
```markdown
# [Feature Name] Implementation Plan

**Goal:** [One-sentence feature description]
**Architecture:** [High-level explanation of changes and file relationships]

## Global Constraints
[Copy verbatim from spec: required versions, names, styling rules]

## Tasks
### Task 1: [Task Name]
* **Files:**
  - Create: `path`
  - Modify: `path:line-range`
  - Test: `path`
* **Interface:** Spec/signature (e.g., `function calculate(val: number): number`)
* **Verification:**
  - [ ] Test case verification (assert inputs/outputs)
  - [ ] Run command: `exact test command` (Expected: PASS)
  - [ ] Commit (after user approval): `git add <files> && git commit -m "feat/fix: <message>"`
```

*If reviewing the plan with a subagent, use `plan-document-reviewer-prompt.md`.*

---

## 2. Execution Phase

Once the plan is ready, announce the execution method: Inline (this session) or Subagent-Driven.

### Option A: Inline Execution (Single Session)
> **Never commit or push without explicit user consent. Present the changes for approval first.**

1. Maintain a checkpoint checklist of the tasks.
2. For each task:
   * Implement the required changes.
   * Verify using the specified test commands.
   * If the user explicitly approved committing, commit exactly the task-related files; otherwise leave changes uncommitted and report the verification result.
3. If blocked or if tests fail repeatedly, stop and check `systematic-debugging`.

### Option B: Subagent-Driven Development (SDD)
Use subagents to isolate the context window, avoiding "attention dilution" on large tasks.

1. **Task Briefing:** Run `scripts/task-brief PLAN_FILE N` to get the task brief.
2. **Context Sharing (No Siloing):** Provide the subagent with the **overall plan architecture/goals** alongside the task brief. Do not hide the global design; subagents need it to write compatible code.
3. **Dispatch Implementer:** Spawn a subagent (using `implementer-prompt.md`) with the brief and architectural context.
4. **Task Review:** Run `scripts/review-package BASE HEAD` to generate the diff. Dispatch a reviewer (using `task-reviewer-prompt.md`). Correct any Critical/Important issues before marking the task complete in your ledger.

---

## 3. Parallel Dispatch

If you face multiple completely independent problem domains (e.g., separate bug fixes in different components), work in parallel:
1. Ensure the agents will not edit the same files or share state.
2. Provide each agent with:
   * Specific scope and target files.
   * Expected inputs, outputs, and tests.
3. Dispatch all agents in a single tool/response batch.
4. Integrate their code, resolve any minor conflicts, and run the full test suite.
