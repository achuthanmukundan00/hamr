# Agent Loop

Hamr uses a bounded model↔tool loop with progressive guardrails designed to survive the messiness of local inference.

## Loop Flow

1. User task is added to the conversation.
2. Model request is assembled with tools, orientation, and memory index.
3. Context budget is checked — multi-stage compaction runs if needed.
4. Model call is dispatched to the provider.
5. Response is parsed: tool calls extracted, reasoning sanitized.
6. Tool calls are executed through local guardrails (ActionExecutor).
7. Tool results are appended (compact format for old results).
8. Loop repeats until completion, error, or budget exhaustion.

## Completion Detection

The loop stops when the model returns a response with **zero tool calls**. Before accepting completion, Hamr runs multiple guards:

- **Planning prose detection** — If the model says "Let me analyze..." or "I'll read..." but emits no tool calls, Hamr injects an action nudge instead of accepting completion. This prevents local models from declaring intent without acting (up to 3 nudges).
- **Verification contract** — In `patch` mode, at least one file must be changed. In `verify` mode, the verification command must pass. Contracts are checked before accepting completion.
- **Status-only output rejection** — Responses like "completed" or "Status: done" are rejected as completions.

## Tool Surface

The model-facing tool names:

| Tool            | Purpose                                                  |
| --------------- | -------------------------------------------------------- |
| `read`          | List files, read a bounded file range, or search text    |
| `edit`          | Exact string replacement in a file                       |
| `write`         | Create a new repo-local text file                        |
| `bash`          | Run terminal commands, including git and verification    |
| `view_image`    | Read image file, return base64 for vision-model analysis |
| `search_memory` | Search conversation history with FTS5 stemming           |
| `save_memory`   | Save a fact to persistent FTS5 memory                   |

## Tool Execution

### Read Cache

Read results are cached per turn. Cache entries are invalidated **only for the specific file** that was mutated (by `edit`, `write`, or `bash` with a known target). Previous behavior cleared the entire cache on any mutation, which forced unnecessary re-reads and collided with the identical-read loop detector.

### Identical-Read Loop Detection

If the model reads the same file at the same line range 5 times without making progress, the turn is terminated with `tool_error`. This prevents the classic "read the same directory listing forever" loop.

### Identical-Bash Loop Detection

If the model runs the same bash command 3 times without completing the task, the turn is terminated. This catches `npm test` loops and similar.

### Read Budget

After 32 reads (50% of the 64-read limit), a warning is injected: "⛔ STOP READING. You have enough context." After 64 reads, further reads are rejected.

## Progressive Escalation

Hamr tracks the read-to-edit ratio across steps. After 8 consecutive steps with zero file mutations (writes/edits), escalating nudges are injected:

| Level | Steps | Nudge |
|-------|-------|-------|
| 1 | 8+ | "You have enough context. Use bash, edit, or write to take action." |
| 2 | 9+ | "⚠️ STOP reading. You MUST use write, edit, or bash now." |
| 3 | 10+ | "🚨 EMERGENCY: zero file changes. Last chance to act." |

Level 3 injects as a high-priority system message. The counter resets on any successful edit or write.

## Truncation Handling

When the model response is cut off by the output token limit (`finish_reason=length`):

- **Content-XML format**: Complete `<tool_call>...</tool_call>` blocks are salvaged and executed. Incomplete XML is dropped.
- **OpenAI format**: Tool calls are dropped (deltas may be incomplete).
- A continuation nudge is injected: "Continue from where you stopped, in smaller pieces."
- After 3 consecutive truncations, the turn fails with `model_error`.

## Context Budget & Compaction

The context budget is resolved from:

```
contextWindowTokens - reservedOutputTokens
```

When the assembled request approaches the budget limit, multi-stage compaction runs:

| Stage | Action |
|-------|--------|
| 0 | Deterministic: strip ANSI, deduplicate lines, collapse whitespace |
| 1 | Normal compaction: keep recent turns, summarize older ones |
| 2 | Reduced tail (60% of stage 1) |
| 2b | Further reduced (40%) |
| 3 | Aggressive: 1200-char hard cap on summaries |
| 4 | Fail-closed: budget_exhausted |

Compaction preserves tool-call/tool-result protocol integrity — no orphaned calls or results.

## Recovery

After a turn fails, `startTurnWithRecovery` classifies the failure scenario and attempts recovery (up to 5 retries):

| Scenario | Recovery |
|----------|----------|
| `empty_response` | Inject "Your last response was empty. Continue..." |
| `infinite_loop` | Inject "You appear stuck. Try a different approach." |
| `bash_failure` | Feed stderr back to model |
| `malformed_tool_call` | Inject format template for correct tool call |
| `context_exhaustion` | Inject "Stop reading files. Take action now." |

## Loop Limits

```toml
[agent]
context_budget_tokens = 131072  # default
max_model_steps = 64
max_tool_calls = 192
```

## Repair Loop

For `hamr run`, after the turn completes, verification runs. If verification fails or the contract is not satisfied, up to `repairAttempts` (default 1) repair turns are executed. Each repair turn gets a targeted task ("Fix the failed verification") and runs through the full loop again.
