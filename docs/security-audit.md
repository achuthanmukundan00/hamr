# Hamr Security Audit — First Public Release

> Audited: 2026-07-10 | Version: 0.3.0 | Auditor: automated + manual review

## Scope

Full codebase audit covering:
- Credential handling & secret leakage
- Input validation & injection vectors
- File system access controls
- Network communication security
- Subagent isolation & privilege boundaries
- Memory/database access patterns

---

## 1. CRITICAL: Credential Leakage in `.hamr.toml`

**Severity: CRITICAL**  
**File:** `.hamr.toml` (lines 13-14)

The repository's own `.hamr.toml` contains **hardcoded Cloudflare Access credentials**:

```toml
[providers.relay.headers]
"CF-Access-Client-Id" = "fe9181e1cad51b266dff2fda0306ec22.access"
"CF-Access-Client-Secret" = "2686515372b3f5de624c31bdd48bd3398ecb3c9d0999d7c6c9ac4448b788e767"
```

**Risk:** These credentials are committed to git history. Anyone who clones the repo can use them to access the relay endpoint.

**Fix:** 
1. Remove these lines from `.hamr.toml` immediately
2. Rotate the Cloudflare Access tokens
3. Add `.hamr.toml` to `.gitignore` (it's already there for the `.hamr` directory but the root config may leak)
4. Use `git filter-branch` or BFG to purge from history

---

## 2. HIGH: API Key Environment Variable Exposure

**Severity: HIGH**  
**File:** `src/config/load-config.ts`, `src/config/load-dotenv.ts`

The configuration system reads API keys from environment variables (`DEEPSEEK_API_KEY`, `ANTHROPIC_API_KEY`, `OPENROUTER_API_KEY`). These are passed through the LLM client chain and could be leaked in error messages or logs.

**Risk:** API keys could appear in:
- Error stack traces shown to users
- Debug logs
- Session persistence

**Mitigation:** 
- API keys should be redacted in all log output
- Error messages should strip credential-bearing headers
- Session serialization must exclude raw API keys

**Status:** Partially mitigated — `src/metrics/CostTracker.ts` strips auth headers before logging, but review needed for all code paths.

---

## 3. HIGH: Command Injection via Bash Tool

**Severity: HIGH**  
**File:** `src/tools/` (bash handler), `src/actions/handlers/`

The `bash` tool allows the model to execute arbitrary shell commands. While this is core functionality, the current implementation should be audited for:
- Shell metacharacter injection
- Command chaining abuse (`&&`, `;`, `|`)
- Path traversal in working directory

**Current mitigations:**
- `rejectsUnsafePaths: true` on file tools
- Safety policy on tool definitions

**Recommendation:** 
- Add a configurable command allowlist/denylist
- Warn on destructive commands (`rm -rf`, `git push --force`)
- Sandbox bash execution in a subprocess with resource limits

---

## 4. MEDIUM: Path Traversal in File Operations

**Severity: MEDIUM**  
**Files:** `src/actions/handlers/read-handler.ts`, `src/actions/handlers/write-handler.ts`

The `read`, `write`, and `edit` tools accept file paths. While `rejectsUnsafePaths` is declared:
- Need to verify that `../` traversal beyond the repo root is blocked
- Symlink following should be controlled
- Absolute paths outside `~/` and the repo should be rejected

**Status:** The tool schema claims `rejectsUnsafePaths: true` — verify implementation.

---

## 5. MEDIUM: Subagent Session Isolation

**Severity: MEDIUM**  
**File:** `src/session/Session.ts` (fork method), `src/orchestration/OrchestrationManager.ts`

Subagents spawned via `dispatch_agents` share:
- The same working tree (no copy-on-write isolation)
- The same FTS5 memory database (inherited via handoff)
- The same filesystem

**Risk:** Parallel subagents writing to overlapping file scopes can corrupt each other's work. The `conflict-detector.ts` catches some cases at file granularity, but line-level conflicts are not detected.

**Current mitigation:** `hasOverlappingFileScopes()` downgrades parallel→sequential when scopes overlap.

**Recommendation:**
- Add optional copy-on-write filesystem overlay for parallel subagents
- Implement line-level conflict detection using diff inspection
- Add subagent resource quota (max file ops, max bash time)

---

## 6. MEDIUM: Memory Database Access

**Severity: MEDIUM**  
**File:** `src/memory/HolographicMemory.ts`, `src/store/EventStore.ts`

The FTS5 SQLite database is shared between parent and child sessions. No access control exists between subagents — any subagent can read/write to any memory entry.

**Recommendation:**
- Namespace memory entries by session/agent ID
- Add optional read-only memory access for subagents
- Validate memory content size limits

---

## 7. LOW: Information Disclosure in Error Messages

**Severity: LOW**  
**File:** `src/llm/client.ts`, `src/commands/chat.ts`

Error messages from LLM providers include endpoint URLs and sometimes partial response data. These could leak internal infrastructure details.

**Status:** Recent fix (commit `dac4d18`) added URL/model to error messages — ensure no secrets are included.

---

## 8. LOW: Dependency Supply Chain

**Severity: LOW**  
**File:** `package.json`

Key dependencies:
- `better-sqlite3` (native binary — verify build provenance)
- `commander` (CLI framework — well-audited)
- `toml` (TOML parser — small surface area)

**Recommendation:** 
- Run `npm audit` / `bun audit` regularly
- Pin dependency versions
- Consider `socket.dev` or similar supply-chain scanner

---

## 9. INFORMATIONAL: .gitleaks-report.json

**Severity: INFO**  
**File:** `.gitleaks-report.json`

A gitleaks report is committed to the repository. This may contain findings about secrets. Ensure it doesn't contain actual secrets.

---

## Summary

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 1 | **Needs immediate fix** (hardcoded CF creds) |
| HIGH     | 2 | Needs review before public release |
| MEDIUM   | 3 | Acceptable for beta, fix in next release |
| LOW      | 2 | Non-blocking |
| INFO     | 1 | Verify |

## Action Items Before Public Release

1. **IMMEDIATE:** Remove hardcoded Cloudflare credentials from `.hamr.toml` and rotate tokens
2. **IMMEDIATE:** Add `.hamr.toml` to `.gitignore` if not already present
3. Audit all logging paths for API key leakage
4. Add bash command safety warnings
5. Verify path traversal protections
6. Implement subagent memory namespace isolation
