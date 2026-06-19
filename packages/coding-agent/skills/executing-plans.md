---
name: executing-plans
description: Use when a written plan exists and the work should be executed step by step with checkpoints
---

# Executing Plans

## Core Rule

Follow the plan in order and keep the work bounded.

## Workflow

1. Read the plan and the current repo state.
2. Execute one task or batch at a time.
3. Verify after each batch.
4. Report exactly what changed and what remains.

## Guardrails

- Do not drift outside the plan.
- If the plan is wrong, stop and fix the plan first.
