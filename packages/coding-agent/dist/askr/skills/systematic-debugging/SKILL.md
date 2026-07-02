---
name: systematic-debugging
description: Use when encountering any bug, test failure, or unexpected behavior, before proposing fixes
---

# Systematic Debugging

Avoid "guess-and-check" programming. Always identify the root cause before implementing fixes.

## 1. Root Cause Investigation

Before editing files to fix a bug:
1. **Read Stack Traces:** Note the exact file path, line numbers, error messages, and codes.
2. **Reproduce the Issue:** Execute the failing test or reproduction script to verify it is consistent.
3. **Verify Component Boundaries:** In multi-layer systems (e.g., API -> database), run a diagnostic command or add logs at component boundaries to trace the bad state.
4. **Trace Data Flow:** Trace variables backward from the failure point to their source. (See [root-cause-tracing.md](root-cause-tracing.md)).

## 2. Hypothesis and Testing

1. Formulate a specific hypothesis: *"I think X is causing Y because Z."*
2. Make the **smallest possible change** to test the hypothesis.
3. If the test passes, proceed to implementation. If not, revert the change and formulate a new hypothesis. Do not layer untested fixes on top of each other.

## 3. Implementation and Prevention

1. **Write a regression test:** Create a test case that captures the bug. (Use [test-driven-development-and-verification](../test-driven-development-and-verification/SKILL.md)).
2. **Apply the fix:** Fix the root cause, not the symptom.
3. **Defense-in-depth:** Add validation checks to prevent invalid inputs from propagating down the call stack. (See [defense-in-depth.md](defense-in-depth.md)).
4. **Triage Repeated Failures:** If 3 or more fix attempts fail, check for environmental issues or question the architectural pattern with the user rather than blindly attempting a 4th fix.

---

## Technical Guides
For specific debugging techniques, refer to:
* **[root-cause-tracing.md](root-cause-tracing.md):** Method for backward tracing of variables.
* **[defense-in-depth.md](defense-in-depth.md):** Strategies for layered data validation.
* **[condition-based-waiting.md](condition-based-waiting.md):** Fixing asynchronous race conditions and flaky tests.
