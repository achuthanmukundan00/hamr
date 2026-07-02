# Testing Anti-Patterns

Load when writing/changing tests, adding mocks, or adding test-only production APIs.

Iron laws:
1. Never test mock behavior.
2. Never add test-only methods to production classes.
3. Never mock without understanding dependencies.

## Mock Behavior

Bad: assert mock exists (`sidebar-mock`, mocked call count unrelated to outcome).
Fix: test real behavior or unmock. If mock needed, do not assert on mock internals unless the unit's contract is the interaction itself.

Gate: before asserting on mock, ask "does this prove real behavior?" If no, delete assertion or unmock.

## Test-Only Production Methods

Bad: `destroyForTest()`, cleanup/lifecycle methods only used by tests.
Fix: move cleanup to test utilities unless production owns that lifecycle.

Gate: before adding production method, grep usage. Only tests? Do not add. Wrong owner? Move.

## Mocking Without Understanding

Bad: mocking high-level method that performs side effects the test depends on.
Fix: run once with real implementation, identify slow/external boundary, mock lowest safe layer.

Ask:
- what side effects does real method have?
- does test depend on them?
- can fake preserve required behavior?

## Incomplete Mocks

Bad: partial response with only fields current assertion uses.
Fix: mirror documented/real shape for all downstream-consumed fields.

If unsure, inspect real docs/example. Partial mocks give false confidence.

## Tests After Implementation

"Ready for testing" means not done. TDD: failing test, code, green, refactor, then claim.

## Mock Too Complex

Signs: setup longer than test, mocks everywhere, test breaks when mock changes, cannot explain why mock needed. Prefer integration test with real components.

## Red Flags

- `*-mock` assertions
- methods only in tests
- mock setup > half test
- "mock to be safe"
- cannot name dependency chain

Bottom line: mocks isolate. They are not the behavior under test.
