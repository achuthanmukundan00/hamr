---
name: test-driven-development-and-verification
description: Use when implementing any feature or bugfix, or before claiming a task is complete
---

# Test-Driven Development and Verification

This skill governs the process of writing code through test feedback and verifying all completions empirically.

## 1. Test-First Implementation

Writing the test first designs the interface and validates that the test suite is actually testing the right behavior.

### Red-Green-Refactor Cycle
1. **RED:** Write one minimal test showing the desired behavior.
2. **Verify RED (Mandatory):** Run the test and confirm it fails because the feature is missing, not due to syntax or environmental errors.
3. **GREEN:** Write the minimal code to make the test pass.
4. **Verify GREEN (Mandatory):** Run the test to ensure it passes and no regressions are introduced.
5. **REFACTOR:** Clean up duplication, improve names, and simplify code structure while keeping tests green.

### LLM-Specific Performance Tweaks
* **No Sunk-Cost Dogma:** Unlike humans who require training discipline, do not delete working code just because it was written before the test. If you have valid code, write the tests to cover it, run them to verify correctness, and adapt the code inline.
* **Single-Turn Batched Actions:** For simple fixes or small features, you may write both the test and the minimal implementation in the same tool call. Then run the test suite once. This saves a turn cycle while preserving the integrity of automated testing.
* **Avoid Over-Mocking:** Test real behavior using real implementations wherever possible. Over-mocking leads to tests that verify the mocks rather than the production code.

---

## 2. Verification Before Claim

Never assert that a task is complete, fixed, or passing without fresh evidence in the current session.

### The Verification Gate
Before making a correctness claim:
1. Identify the verification command (e.g., test runner, compiler, linter).
2. Run the command and inspect the stdout, stderr, and exit code.
3. Reference the exact command and output in your response to the user.

### Verification Mapping
* **Tests pass:** Test runner output showing 0 failures.
* **Lint clean:** Linter output showing 0 errors.
* **Build succeeds:** Build command exit code is 0.
* **Bug fixed:** The reproduction script or test case now passes.
* **Requirements met:** Line-by-line checklist verification against the spec/plan.
