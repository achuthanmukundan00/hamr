# Root Cause Tracing

When bug appears deep in stack, trace backward to original trigger. Fix source, not symptom.

## Process

1. Observe symptom and exact failing operation.
2. Find immediate cause: code/value directly causing failure.
3. Ask "who called this?" and "what value was passed?"
4. Walk up call chain until bad value/decision originates.
5. Fix there.
6. Add defense-in-depth where bad data crossed layers.

## Instrument

When manual trace fails, log before dangerous operation:

```typescript
console.error("DEBUG dangerous op", {
  value,
  cwd: process.cwd(),
  env: process.env.NODE_ENV,
  stack: new Error().stack,
});
```

In tests, use `console.error` so output appears. Include path/cwd/env/timestamp/stack. Log before operation, not after failure.

## Test Pollution

Use `find-polluter.sh`:

```bash
./find-polluter.sh '.git' 'src/**/*.test.ts'
```

Runs tests one-by-one, stops at first polluter.

Rule: never fix only where error appears unless tracing is impossible and you document why.
