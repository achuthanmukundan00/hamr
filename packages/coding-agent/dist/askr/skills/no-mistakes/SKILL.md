---
name: no-mistakes
description: Use when the user asks to run no-mistakes, gate, ship, validate changes, push safely, or do work and then validate it through the no-mistakes pipeline
user-invocable: true
---

# no-mistakes

`no-mistakes` is an AXI gate for committed code: intent, rebase, review, test, docs, lint, push, PR, and CI. It prints TOON on stdout and progress on stderr.

## Modes

- **Validate-only:** user already has committed changes; run the gate.
- **Task-first:** do the requested work, ask for explicit commit consent, commit only task-related files on a feature branch if approved, then run the gate with the user's request as `--intent`. If commit consent is not granted, stop after manual validation and report that `no-mistakes` requires committed changes.

## Preconditions

- **`no-mistakes` CLI must be available.** The askr package ships a launcher at `../../bin/no-mistakes.js` that tries the native binary and falls back to `npx -y @skaft/no-mistakes`. Set `ASKR_NO_MISTAKES_BIN` to override the binary path. If neither is available, inform the user and offer manual validation alternatives (run tests, lint, build manually).
- Work is committed, or the user has explicitly approved creating the required task-scoped commit.
- Current branch is not the default branch.
- Repo has `no-mistakes init` already; if not, run it when appropriate.
- Start with `no-mistakes axi` to see active runs. Resume/drive a current-branch run instead of starting or aborting over it; leave other-branch runs alone. Re-running `axi run` may reattach to a matching in-flight run.

## Run Loop

```bash
no-mistakes axi run --intent "<what the user set out to accomplish, with key constraints and decisions>"
```

- Long calls are normal; do not cancel because review/test/CI is slow. Use `no-mistakes axi status` from another call if needed; `awaiting_agent: parked` means respond, not wait.
- If output has `gate:`, the pipeline is waiting for you. Read the actual `findings[...]` header and `action` values.
- Respond until an `outcome:` appears. On `failed`/`cancelled`, fix the named blocker, ask for commit consent if a new commit is needed, commit on the same branch only when approved, then start a fresh run or `no-mistakes rerun`.
  ```bash
  no-mistakes axi respond --action approve
  no-mistakes axi respond --action fix --findings <id1,id2> --instructions "<optional>"
  no-mistakes axi respond --action skip
  ```

## Finding Policy

- `auto-fix`: you may authorize `respond --action fix` on your judgment. Review auto-fix may be disabled by config; parked review findings still need a response.
- `no-op`: informational; approve when nothing else blocks.
- `ask-user`: stop and relay each finding's id, file, and description verbatim. Ask the user whether to fix, approve, or skip, then translate their choice into `respond`.
- Only use `--yes` when the user gave clear standing consent to drive every gate unattended.

## Do Not

- Do not manually edit code while a run is active; the pipeline owns fixes through `respond --action fix`.
- Do not abort/rerun to bypass a gate. Abort/rerun only between runs after `failed`/`cancelled` or when intentionally discarding a run.
- Do not wait for merge after `checks-passed`; tell the user the PR is ready.
- Do not hand-rebase a still-monitored PR; no-mistakes handles conflicts while its monitor is active.

## Useful Commands

```bash
no-mistakes axi
no-mistakes axi status
no-mistakes axi logs --step <name> --full
no-mistakes axi abort
no-mistakes axi abort --run <id>
```
