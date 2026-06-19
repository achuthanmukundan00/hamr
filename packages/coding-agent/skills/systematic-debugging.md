---
name: systematic-debugging
description: Use when a bug, test failure, or unexpected behavior needs root-cause analysis before fixing
---

# Systematic Debugging

## Core Rule

Find the root cause before changing code.

## Workflow

1. Reproduce the problem.
2. Reduce it to the smallest failing case.
3. Trace the data or control flow.
4. Fix the root cause, not the symptom.
5. Re-run the original repro and related tests.

## Guardrails

- If you cannot explain the failure, do not patch it yet.
- Prefer instrumentation and logs over guessing.
