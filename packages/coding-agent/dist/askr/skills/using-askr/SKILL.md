---
name: using-askr
description: Use at session start or after compaction to locate the right agentic skills
---

# Using AskR

AskR is a compact skill router. Use the matching skill before improvising; do not load skills for trivial chat.

| Need | Skill |
|------|-------|
| Ambiguous feature, product/UX/TUI design, architecture, visual plan, comparison, artifact review | `collaborative-design` |
| Multi-step implementation plan, inline execution, subagents, or parallel work | `planning-and-execution` |
| Feature/bug implementation or any final correctness claim | `test-driven-development-and-verification` |
| Bug, failing test, flake, or unexpected behavior | `systematic-debugging` |
| Branch/worktree setup, review handling, merge/PR/cleanup | `git-and-branch-lifecycle` |
| Agent-facing CLI/tool design or review | `axi` |
| User asks to gate/ship/validate safely with no-mistakes | `no-mistakes` |

Principle: keep AskR thin; use external AXIs like `lavish-axi` and `no-mistakes` for heavyweight collaboration and validation.
