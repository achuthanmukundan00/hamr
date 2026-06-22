# Defense-In-Depth Validation

After root cause is known, make invalid data hard to pass through.

Use when bug came from invalid value/state moving across layers.

## Layers

1. Entry/API validation: reject bad input early.
2. Business validation: assert operation-specific invariants.
3. Environment guard: prevent dangerous context-specific action (tests, prod, path).
4. Debug instrumentation: log enough context for next failure.

## Apply

1. Trace data flow.
2. Mark every checkpoint.
3. Add validation at each meaningful layer.
4. Test bypasses: if layer 1 missed, layer 2 catches; env guard catches dangerous case.

Do not stop at first validation if other paths/mocks/refactors can bypass it.
