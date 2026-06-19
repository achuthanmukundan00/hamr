---
name: finishing-a-development-branch
description: Use when implementation is complete and the branch needs verification, cleanup, and handoff
---

# Finishing a Development Branch

## Core Rule

Verify first, then decide how to integrate.

## Workflow

1. Run the full verification suite.
2. Check the branch state and workspace type.
3. Present the available completion paths.
4. Execute the chosen path.
5. Clean up the temporary workspace.

## Guardrails

- Do not merge or PR until the repo is green.
- Do not leave stale worktrees behind.
