# Hamr Security Audit

> Last updated: 2026-06-21 | Auditor: deep code audit (automated + manual review)
>
> This document cites **real** file paths under `packages/coding-agent/src/...` and
> line numbers verified against the working tree. Each control is marked
> **verified** (with a test) or **known-gap**. Earlier revisions of this file
> referenced paths that did not exist and a `rejectsUnsafePaths` flag that was
> never implemented — those have been corrected.

## Scope

Full codebase audit covering:
- Credential handling & secret leakage
- Input validation & injection vectors
- File system access controls (path confinement)
- Network communication security (HTTP proxy, provider auth)
- Subagent isolation & privilege boundaries
- Memory/database access patterns

---

## 1. CRITICAL: Leaked Cloudflare Access credentials in git history

**Severity: CRITICAL**
**File:** `.hamr.toml.bak` (was tracked, commit `118655e`)

A tracked backup file contained live, distinct relay credentials:

```toml
api_key = "618831e2379ca68245448dc885879fa7747917ad68ec6658f111458c42ef5952"
[providers.relay.headers]
CF-Access-Client-Id     = "4ca96e3e019a23e98b4db32af0c5e7c1.access"
CF-Access-Client-Secret = "8c3d3e185d7e0993680642160635d582a6e7baa5427fef26c86f5335f77d6da7"
base_url = "https://ai.watchyourtemper.com/v1"
```

**Risk:** Anyone cloning the public repo could impersonate the Skaft relay client.

**Fix applied (issue #26):**
1. `git rm --cached .hamr.toml.bak` and deleted the file from the working tree.
2. Hardened `.gitignore` to ignore `.hamr.toml`, `.hamr.toml.bak`, and `.hamr.toml.bak-*`.
3. **Still required:** Rotate all three leaked values at Cloudflare Access + the relay admin, and purge the blob from history with `git filter-repo` / BFG (commit `118655e`). This rotation/purge is an out-of-band admin action, not a code change.

**Status:** File removed from tracking; rotation + history purge **outstanding**.

---

## 2. HIGH: Path-traversal / host-compromise via unconfined file tools

**Severity: HIGH**
**Files:** `packages/coding-agent/src/core/tools/{write,read,edit}.ts`

The file tools resolved paths with `resolveToCwd()` (`core/tools/path-utils.ts`) which only joins
paths against the cwd — it performed **no** boundary check. A prompt-injected or jailbroken model
could `read ~/.ssh/id_rsa`, `~/.aws/credentials`, `~/.config/hamr/auth.json`, and establish
persistence by writing `~/.ssh/authorized_keys` or `~/.bashrc`.

> Note: a previous revision of this doc claimed a `rejectsUnsafePaths: true` flag existed on the
> file tools. That flag was **never implemented** (`grep -rn rejectsUnsafePaths packages` returns
> zero hits).

**Fix applied (issue #28):** Added `core/tools/path-guard.ts` — a `PathGuard` that applies a hard
denylist of credential and persistence locations, on by default, configurable via `PathGuardOptions`
on `ReadToolOptions` / `WriteToolOptions` / `EditToolOptions`.

- **Writes blocked:** `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.config/hamr`, `~/.hamr`, `/etc`,
  cron/at spools (`/var/spool/cron`, `/var/spool/at`, `/var/at`, `/var/cron`), `~/.config/systemd`,
  `~/Library/LaunchAgents`, `~/Library/LaunchDaemons`, `.git/hooks/**`, shell rc files
  (`~/.bashrc`, `~/.zshrc`, `~/.profile`, …), `authorized_keys`.
- **Reads blocked:** SSH private keys (`~/.ssh/id_*`), `~/.aws/credentials`, `~/.aws/config`,
  `~/.config/hamr/auth.json`, `~/.hamr/auth.json`, `~/.netrc`, `~/.gnupg/**`.
- macOS `/var/folders` temp and project-internal paths remain allowed.
- `allowedPaths` lets users opt specific roots back in; `enabled: false` disables the guard.

**Verified:** `test/path-guard.test.ts` (20 cases). **Known gap:** the `bash` tool itself still
executes arbitrary commands by design (core functionality); path confinement covers only the
read/write/edit file tools, not shell-spawned commands.

---

## 3. HIGH: HTTP proxy from settings silently MITM'd all LLM traffic

**Severity: HIGH**
**File:** `packages/coding-agent/src/core/http-dispatcher.ts` (`applyHttpProxySettings`, applied in `src/main.ts:509,781`)

`httpProxy` from `settings.json` / `.hamr.toml` was applied to `HTTP_PROXY`/`HTTPS_PROXY` with no
validation and no warning, before any model call. A single malicious config line routed every
provider request — carrying `Authorization: Bearer <key>` and `CF-Access-Client-*` headers —
through an attacker-controlled proxy.

**Fix applied (issue #29):** Added `validateProxyUrl()` (rejects non-http(s) schemes, malformed
URLs, missing hosts) and `warnProxyActive()` (prints a prominent warning naming the proxy host).
`applyHttpProxySettings` now validates then warns on apply.

**Verified:** `test/path-guard.test.ts` (validateProxyUrl cases) and `test/http-dispatcher.test.ts`.
**Known gap:** credential headers are still sent through the configured proxy; a future hardening
could strip/redirect auth headers over untrusted proxies.

---

## 4. HIGH: Subagent child-config temp file leaked API key / CF tokens

**Severity: HIGH**
**File:** `packages/coding-agent/src/hamr/extensions/subagents.ts` (~line 720-740)

The parent serialized its full auth (`apiKey`, `apiHeaders` including `CF-Access-Client-*`, `apiEnv`)
into `os.tmpdir()/hamr-config-<uuid>.json` written with default (world-readable) permissions.
Cleanup only ran in the child's `close`/`error` handlers, so a parent SIGKILL/crash orphaned the
secret in `/tmp` forever.

**Fix applied (issue #27):**
- Written with `{ encoding: "utf-8", mode: 0o600 }` + `chmodSync(0o600)` to defeat umask.
- Registered with `registerOrphanedConfigForCleanup()`; a process-exit hook (`exit`/`SIGINT`/`SIGTERM`)
  unlinks any still-registered config paths and calls `killTrackedDetachedChildren()`.

**Status:** verified by code review; no dedicated test yet (orphan-on-crash is hard to unit-test).

---

## 5. MEDIUM: Subagent tree budget was non-atomic / leaked on early failure

**Severity: MEDIUM**
**File:** `packages/coding-agent/src/hamr/extensions/subagents.ts` (`treeBudgetRemaining` ~line 57, 1985-1997)

The budget check+decrement is synchronous (so concurrent tool calls cannot interleave at that
point — the original "non-atomic race" concern was a **possible false positive**). However, the
decrement happened *before* a hard-max validation that could early-return, and any thrown exception
before workers spawned permanently leaked the reserved slots.

**Fix applied (issue #30):**
- Moved the hard-max task-count validation **before** the budget reservation.
- Added a `catch` around the execution that refunds `taskCount - run.workers.size` (unspawned slots)
  when the run aborts before/during spawning.

**Status:** verified by code review.

---

## 6. MEDIUM: Orphaned subagent processes when parent is killed

**Severity: MEDIUM**
**File:** `packages/coding-agent/src/hamr/extensions/subagents.ts` (~line 749-830, 610-670)

Worker children were spawned with `detached:false`, no `unref()`, and no parent-exit cleanup. A
parent SIGKILL/OOM/crash left `hamr --mode json` children running, continuing to call the LLM API
and holding budget slots.

**Fix applied (issue #31):**
- Both spawn sites (worker + bash fast-path) now use `detached: true` (Unix), `trackDetachedChildPid`
  on spawn, `untrackDetachedChildPid` on close, and `killProcessTree(pid)` on abort.
- The parent-exit hook installed by `registerOrphanedConfigForCleanup` also calls
  `killTrackedDetachedChildren()` on `exit`/`SIGINT`/`SIGTERM`.

**Status:** verified by code review.

---

## 7. MEDIUM: Bash/subagent temp output files were world-readable

**Severity: MEDIUM**
**Files:** `core/tools/output-accumulator.ts:215`, `core/bash-executor.ts:70`, `subagents.ts` (run.json + result writes)

Temp files holding command output (which may contain secrets the model read, e.g. `cat ~/.ssh/id_rsa`)
were created without restrictive permissions.

**Fix applied (issue #32):**
- `output-accumulator.ts` and `bash-executor.ts` now create temp files with `openSync(path, "w", 0o600)`
  + `createWriteStream(path, { fd })` + `chmodSync(0o600)`.
- `bash-executor.ts` now `await`s the stream's `finish` before returning `fullOutputPath`, so the
  file is fully flushed when handed back (previously relied on lazy file creation + polling).
- Subagent `run.json` / result files written with `{ encoding: "utf-8", mode: 0o600 }`; run log dir
  created with `mode: 0o700` + `chmodSync(0o700)`.

**Verified:** `test/tools.test.ts` (bash full-output truncation tests).

---

## 8. MEDIUM: Extension execCommand had no default timeout

**Severity: MEDIUM**
**File:** `packages/coding-agent/src/core/exec.ts`

`execCommand` (used by extension `api.exec`) only applied a timeout when explicitly passed; a hung
extension subprocess could block the agent forever.

**Fix applied (issue #33):** Added `DEFAULT_EXEC_TIMEOUT_MS` (10 min) applied when no timeout is
provided. A caller may pass `0` to explicitly disable.

**Status:** verified by code review.

---

## 9. MEDIUM: Auth-storage lock retry busy-wait spun CPU

**Severity: MEDIUM**
**File:** `packages/coding-agent/src/core/auth-storage.ts` (`acquireLockSyncWithRetry`)

The synchronous lock retry used a `while (Date.now() - start < delayMs) {}` busy-spin (up to 2s × 25
attempts), pinning a CPU core and blocking the event loop during token-refresh contention.

**Fix applied (issue #34):** Replaced the busy-spin with `syncSleepMs()` using `Atomics.wait` on a
`SharedArrayBuffer`-backed `Int32Array` (a genuine blocking wait without CPU burn), with a
bounded busy-wait fallback only when `SharedArrayBuffer` is unavailable.

**Verified:** `test/auth-storage.test.ts` (31 cases pass).

---

## 10. LOW: Provider auth storage

**Severity: LOW**
**File:** `packages/coding-agent/src/core/auth-storage.ts`

`auth.json` is written with `mode: 0o600`, parent dir `0o700`, and `proper-lockfile` locking (the
sync path now uses the non-busy sleep above; the async path already used `lockfile.lock` with
retries/stale config). OAuth flow is handled by `@hamr/ai`.

**Status:** verified, no change required.

---

## 11. INFORMATIONAL: Subagent session/memory isolation

**Severity: INFO**
**File:** `packages/coding-agent/src/hamr/extensions/subagents.ts`, `core/session-manager.ts`

Subagent workers run in isolated child `hamr --mode json -p` processes. They share the parent's
working tree and (via handoff) the FTS5 memory database. Parallel workers writing to overlapping
file scopes can corrupt each other's work; line-level conflicts are not detected.

**Status:** acceptable for the extension platform's trust model (workers are spawned by the trusted
model). Not addressed in this pass.

---

## 12. INFORMATIONAL: SQL/FTS5 injection

**Severity: INFO**
**File:** `packages/coding-agent/src/hamr/memory/FactStore.ts`

`FactStore` uses parameterized prepared statements (`@query`, `?` placeholders) and an FTS5 query
sanitizer (`[^\w\s*\-"()]` stripped). No SQL/FTS injection path found. The dynamic `IN (...)` clause
for retrieval-count updates is built from DB-sourced integer IDs — safe.

**Status:** verified, no change required.

---

## Summary

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 1 | File removed; rotation + history purge outstanding |
| HIGH     | 3 | Fixed (#27, #28, #29) |
| MEDIUM   | 5 | Fixed (#30, #31, #32, #33, #34) |
| LOW      | 1 | No change required |
| INFO     | 2 | Noted, acceptable |

## Outstanding action items

1. **Rotate** the leaked CF-Access + relay API key values at Cloudflare / relay admin.
2. **Purge** the `.hamr.toml.bak` blob from git history (`git filter-repo` / BFG), then force-push.
3. Consider stripping/redirecting auth headers over untrusted HTTP proxies.
4. Consider line-level conflict detection for parallel subagents writing overlapping files.
