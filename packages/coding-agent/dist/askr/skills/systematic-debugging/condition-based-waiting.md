# Condition-Based Waiting

Wait for condition, not guessed time.

Use when tests use `sleep`, `setTimeout`, `time.sleep`, are flaky, timeout under load, or wait for async completion.

Do not replace real timing tests (debounce/throttle). If arbitrary delay is required, document known interval and first wait for trigger.

## Pattern

Bad:

```typescript
await new Promise(r => setTimeout(r, 50));
expect(getResult()).toBeDefined();
```

Good:

```typescript
await waitFor(() => getResult() !== undefined, "result");
```

Helper:

```typescript
async function waitFor<T>(fn: () => T | undefined | null | false, label: string, timeoutMs = 5000): Promise<T> {
  const start = Date.now();
  while (true) {
    const v = fn();
    if (v) return v;
    if (Date.now() - start > timeoutMs) throw new Error(`Timeout waiting for ${label}`);
    await new Promise(r => setTimeout(r, 10));
  }
}
```

Use fresh getter inside loop. Always timeout. Poll about 10ms, not 1ms.

Examples: event exists, state ready, count >= N, file exists, `obj.ready && obj.value > 10`.

Full helpers: `condition-based-waiting-example.ts`.
