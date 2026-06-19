# Hamr Production Readiness Plan

> Historical planning note. This compares Hamr against an older Pi-era codebase and is kept for context only.
> It does not describe the current docs site or a frozen implementation target.

Based on a thorough comparison with `@earendil-works/pi-coding-agent v0.79.1`.

## Critical Gaps (Must Fix)

### 1. Auth Error Guidance — Missing
**Pi**: `formatNoApiKeyFoundMessage()`, `formatNoModelsAvailableMessage()`, `formatNoModelSelectedMessage()` with provider-specific help text linking to docs.
**Hamr**: Throws raw `Error("No API key configured for ...")` — no user-actionable guidance.

### 2. Config Validation Diagnostics — Missing
**Pi**: `SettingsManager.drainErrors()` collects scope-aware errors (global vs project), `collectSettingsDiagnostics()` + `reportDiagnostics()` with colored output.
**Hamr**: `loadProjectConfig()` returns errors but they're barely surfaced. No diagnostic aggregation.

### 3. Provider Attribution Headers — Missing
**Pi**: Sends `HTTP-Referer`/`X-Title` to OpenRouter, `X-BILLING-INVOKE-ORIGIN` to NVIDIA, `User-Agent` to Cloudflare.
**Hamr**: Only hardcodes OpenRouter headers in default config — no dynamic attribution.

### 4. User-Facing Error Surface — Poor
**Pi**: Every error has a user-actionable message with next steps (e.g., "Run `/login {provider}` to re-authenticate").
**Hamr**: Raw provider errors with stack traces leak to users via `process.stderr`.

### 5. Process Lifecycle — Unmanaged
**Pi**: Suppresses Node warnings, manages stdout takeover for TUI, clean shutdown hooks.
**Hamr**: `process.exit(1)` scattered in TUI code, no stdout/stderr management.

### 6. Startup Configuration Wizard — Missing
**Pi**: `showStartupSelector()` / `showStartupInput()` for first-run config, model selection.
**Hamr**: Just launches with default config — no guided first-run experience.

### 7. Telemetry/Install Tracking — Missing
**Pi**: `isInstallTelemetryEnabled()` with configurable telemetry flag, used for provider attribution.
**Hamr**: Only TUI performance stats, no install/usage telemetry.

### 8. HTTP Dispatcher Configuration — Missing
**Pi**: `configureHttpDispatcher()` customizes undici with idle timeout, DNS caching, redirects.
**Hamr**: Uses global `fetch()` with no dispatcher tuning.

### 9. Self-Update / Version Check — Missing
**Pi**: Full self-update infrastructure (detectInstallMethod, makeSelfUpdateCommand, version check).
**Hamr**: No update notification or self-update capability.

### 10. Project Trust — Missing
**Pi**: `ProjectTrustStore` + `resolveProjectTrusted()` — users must approve project settings.
**Hamr**: Loads all local config without trust prompts.

### 11. Package Distribution — Minimal
**Pi**: Ships `docs/`, `examples/`, `CHANGELOG.md`, shrinkwrap file in npm tarball.
**Hamr**: Ships only `dist/` and `README.md` — no docs, no examples, no changelog.

## Implementation Plan

### Phase 1 — Error & Auth Surface (HIGH)
- [ ] Create `src/commands/auth-guidance.ts` with user-actionable error formatters
- [ ] Add `formatNoApiKeyFoundMessage`, `formatNoModelsAvailableMessage`, `formatNoModelSelectedMessage`
- [ ] Integrate into LLM client error paths
- [ ] Add `--api-key` CLI flag support

### Phase 2 — Diagnostics & Validation (HIGH)
- [ ] Enhance `src/agent/diagnostics.ts` with structured diagnostic aggregation
- [ ] Add diagnostic collection at startup with colored report
- [ ] Improve config validation error messages with file paths and line numbers

### Phase 3 — Provider Attribution (MEDIUM)
- [ ] Create provider-attribution with dynamic headers per provider
- [ ] Add telemetry flag (env var `HAMR_TELEMETRY`) for install tracking
- [ ] Wire attribution headers into LLM request pipeline

### Phase 4 — Process & Lifecycle (MEDIUM)
- [ ] Clean up `process.exit()` calls in TUI code
- [ ] Add proper stdout/stderr management
- [ ] Add version check/update notification
- [ ] Suppress Node warnings

### Phase 5 — Package Excellence (LOW)
- [ ] Include `docs/*.md` and `examples/` in npm package
- [ ] Add `CHANGELOG.md` to files array
- [ ] Improve README with setup troubleshooting section
