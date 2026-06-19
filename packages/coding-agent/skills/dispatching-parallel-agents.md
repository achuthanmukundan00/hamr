---
name: dispatching-parallel-agents
description: Use when 2+ independent tasks can be worked on without shared state or sequential dependencies
---

# Dispatching Parallel Agents

## Core Rule

If tasks are independent, split them across fresh agents.

## Workflow

1. Group work by shared files, shared state, or dependency chains.
2. Give each agent one narrow domain.
3. Keep the context lean and isolated.
4. Merge the results after each agent returns.

## Guardrails

- Do not parallelize related changes.
- Do not let agents inherit irrelevant context.
