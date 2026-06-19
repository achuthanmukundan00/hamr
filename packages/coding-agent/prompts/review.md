---
description: Review local changes, a PR, or a codebase scan with CodeRabbit-style analysis
argument-hint: "[local|pr|scan] [PR-number or path]"
---
Use the CodeRabbit-style review format.

## Modes

- `local`: review the working tree diff
- `pr`: review a GitHub PR diff
- `scan`: deep-scan a path or the full tree

For any mode:
1. Gather the diff or scope
2. Read the changed files for context
3. Report findings by severity with file:line references
4. Put issues first, then strengths, then a short verdict

If the user wants fixes, apply them one at a time and re-verify after each batch.
