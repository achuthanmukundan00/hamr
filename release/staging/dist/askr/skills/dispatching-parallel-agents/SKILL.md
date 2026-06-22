---
name: dispatching-parallel-agents
description: Use when facing 2+ independent tasks that can be worked on without shared state or sequential dependencies
---

# Parallel Agents

Use one subagent per independent problem domain. Saves time, preserves focus.

Use when:
- 2+ failures/tasks are independent.
- Fixing one cannot affect others.
- Agents will not edit same files/state.

Do not use when failures are related, full-system context is needed, root cause unknown, or shared resources conflict.

## Pattern

1. Group by domain: file, subsystem, bug class.
2. For each agent, provide:
   - exact scope
   - goal
   - constraints
   - relevant errors/tests
   - expected return: root cause, changes, tests
3. Dispatch all agents in one response/tool batch. One per response is sequential.
4. When they return, read summaries, inspect diffs, resolve conflicts, run full suite.

## Prompt Shape

```text
Fix <specific file/subsystem>.
Failures:
- <test/error>
- <test/error>

Task:
1. Read relevant code/tests.
2. Identify root cause.
3. Fix root cause; do not mask with timeouts unless root cause is timeout policy.
4. Run <tests>.

Constraints: <no unrelated files/refactors/etc>.
Return: root cause, files changed, tests run/output.
```

## Red Flags

- "Fix all tests" -> too broad.
- No error/context -> agent guesses.
- No constraints -> agent sprawls.
- No output contract -> cannot integrate.
- Agents edit same files -> run sequentially or split differently.
