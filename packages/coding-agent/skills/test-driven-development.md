---
name: test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---

# Test-Driven Development

## Core Rule

Write the failing test first.

## Cycle

1. Add the smallest test that proves the behavior.
2. Run it and confirm it fails for the right reason.
3. Write the smallest code that makes it pass.
4. Run the test again.
5. Refactor only after green.

## Guardrails

- Delete any code written before the test if it helped you cheat.
- Prefer regression tests for bugs.
- Keep the test focused on the behavior the user cares about.
