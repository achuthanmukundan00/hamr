# Implementer Prompt Template

```text
Subagent:
  description: "Implement Task N: <name>"
  model: <REQUIRED>
  prompt: |
    Implement Task N: <name>.

    Read first: <BRIEF_FILE>. It is your requirements source.
    Work dir: <DIR>.
    Context: <where this fits, prior interfaces/decisions only>.
    Report file: <REPORT_FILE>.

    Before work, ask if requirements, acceptance criteria, deps, or approach are unclear.
    During work, ask on surprises. Do not guess.

    Job:
    1. Implement exactly the brief.
    2. Follow TDD if required.
    3. Run focused tests while iterating; full relevant suite before commit.
    4. Commit work.
    5. Self-review, fix issues found.
    6. Write report file.

    Code:
    - follow plan file structure and existing patterns
    - one responsibility per file/interface
    - no unrelated refactor or overbuild
    - if task needs unplanned architecture/split/broad context, stop with BLOCKED/NEEDS_CONTEXT

    Self-review:
    - complete vs brief?
    - YAGNI?
    - names clear?
    - tests verify behavior, not mocks?
    - test output clean?

    Report file must include:
    - implemented/attempted
    - tests run + results
    - TDD evidence if required: RED command/output/why expected; GREEN command/output
    - files changed
    - self-review findings/fixes
    - concerns

    Chat reply only, under 15 lines:
    - Status: DONE | DONE_WITH_CONCERNS | BLOCKED | NEEDS_CONTEXT
    - commits: short SHA + subject
    - tests: one-line summary
    - concerns/blocker/context need
    - report path

    Use DONE_WITH_CONCERNS when complete but doubtful. BLOCKED when unable.
    NEEDS_CONTEXT when missing info. Never hide uncertainty.

    If fixing review findings later, append fix + test evidence to same report file.
```
