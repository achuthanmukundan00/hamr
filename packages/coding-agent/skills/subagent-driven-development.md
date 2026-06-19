---
name: subagent-driven-development
description: Use when executing an implementation plan with independent tasks in the current session
---

# Subagent-Driven Development

## Core Rule

Fresh subagent per task, with review between tasks.

## Workflow

1. Give each task a narrow prompt and clear acceptance criteria.
2. Run the task in isolation.
3. Review for spec compliance first.
4. Review for code quality second.
5. Move to the next task only after the current one is verified.

## Guardrails

- Do not keep the same subagent context across unrelated tasks.
- Do not skip the review steps unless the task is trivial.
