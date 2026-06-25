# Changelog

## [0.6.7] - 2026-06-24

### Fixed

- **npm self-update `--prefix` used the wrong directory.** The v0.6.5 fix correctly omitted `-g` when `--prefix` was set, but passed the npm global prefix (`/opt/homebrew`) instead of the `node_modules` parent (`/opt/homebrew/lib`). Without `-g`, npm installs to `{--prefix}/node_modules/` rather than `{--prefix}/lib/node_modules/`, so the updated binary landed one directory level too high and the old binary remained on `PATH`. Fixed by using `dirname(root)` (the `node_modules` parent directory) as the `--prefix` value.
- **Bumped `undici` from 8.3.0 to 8.5.0** to address 7 high-severity advisories (TLS bypass, header injection, cache disclosure, DoS vectors).

## [0.6.6] - 2026-06-24

### Changed

- **Memory context injection: query-triggered recall replaces one-shot injection.** Previously, memory context was injected once per session based on suggested search terms derived from word frequency in stored entries. Now `extractMessageSearchTerms()` derives terms from the *current* user message. Generic greetings and acknowledgments ("hi", "ok", "thanks") produce no terms and self-silence, preventing irrelevant past context from hijacking the turn. Topical queries naturally yield search terms that retrieve relevant cross-session entries via FTS5. Content-hash de-duplication remains but now allows different queries through on the same session. The "Auto-retrieved context" heading is changed to "You may have prior context on this:" to better signal the model. `computeMemoryInjection()` is extracted as a pure function for testability. Removed the `injectedSessions` one-shot gate — injection is now gated purely by message-derived terms and content-hash change detection.
- **DeepSeek V4 Flash pricing and max-tokens updated** to match current OpenRouter rates: input 0.089/MTok (was 0.09), output 0.224/MTok (was 0.18), cache-read 0.0266/MTok (was 0.02), max output tokens 4,096 (was 65,536).
- **Generic parser now falls back to Qwen3 XML for unrecognized model families.** When `<tool_call>` blocks contain non-JSON content, the generic parser delegates to the Qwen3 XML parser. This catches Qwen derivatives (Apodex and other unrecognized families) that emit `<function=…><parameter=…>…</parameter></function>` XML, preventing silent tool-call failures.
- **Subagents: empty `toolNames` default now includes core tools.** Child workers no longer start with an empty tool list when the user omits explicit `tools`. Defaults to `["read", "bash", "edit", "write"]`.
- **Subagents: empty_output validation hardened.** The `v.warnings.some(...)` check now uses optional chaining (`v?.warnings?.some(...)`) to guard against undefined warnings arrays, preventing a potential crash on malformed validation payloads.

## [0.6.5] - 2026-06-24

### Fixed

- **npm self-update on Homebrew macOS.** `-g` overrides `--prefix` in npm, so `npm --prefix /opt/homebrew install -g` silently installed to npm's global prefix instead of the inferred Homebrew prefix. The `-g` flag is now omitted when `--prefix` is explicitly set from the inferred install prefix.

## [0.6.4] - 2026-06-24

### Added

- **3 new HuggingFace models in the registry.** Qwen3.6-27B, MiMo-V2.5-Pro, Step-3.7-Flash added with full capability descriptors (context window, reasoning, thinking levels, pricing).

### Fixed

- **Subagent model inheritance: child processes now use the parent's model instead of falling back to a relay default.** The root cause was a priority bug in `createAgentSessionFromChildConfig` where `options.model` (resolved from `scopedModels[0]` in `buildSessionOptions`) overrode the parent-serialized `HamrChildConfig` model. Every subagent spawned without an explicit `--model` flag would pick the first relay-discovered model regardless of what the parent was using. Fixed by (a) giving the child config model priority over `options.model` in `createAgentSessionFromChildConfig`, and (b) skipping default model resolution in `buildSessionOptions` when `HAMR_CHILD_CONFIG` is set.
- **Non-cloud providers are now blocked from dispatching parallel subagents.** Relay and local endpoints cannot support child-process model inheritance yet, so `delegate_subagents` now returns a clear error ("Relay/local provider cannot dispatch subagents") instead of silently spawning workers that fall back to whatever model the relay endpoint discovers first.
- **Robust `parentConfig` serialization.** The `modelCompat` field in the child config uses `safeJsonClone` (graceful fallback on BigInt/circular-ref serialization failures) instead of a bare `JSON.parse(JSON.stringify(...))` that could silently drop the entire parent config payload.
- **Removed dead `ENV_MAX_LOCAL_CONCURRENCY`.** The non-cloud guard makes the per-provider concurrency cap redundant — cloud providers always use `ENV_MAX_CONCURRENCY` and non-cloud providers are blocked before reaching the concurrency limiter.
- **Interactive mode: truncated status lines.** The subagent status widget and persistent editor footer now use `truncateToWidth` from `@hamr/tui` instead of naive `slice(0, width)` which could break ANSI sequences. Removed the local `visibleWidth` helper in `persistent-editor.ts`.

### Changed

- **Footer model display simplified.** The model name in the footer bar now always shows the parenthesized provider prefix (`(DeepSeek) deepseek-v4-pro`) instead of a compact provider-slash form. The "Working..." status text was shortened to "Working".
- **Pricing and max-tokens adjustments.** Claude Opus 4.7 cache-read cost corrected from 0.49 to 0.182 and max-tokens to 4096 (OpenRouter's Chat Completions API returns short reasoning). Qwen3.5 7B tuned to 0.05 cache-read and 81,920 max-tokens. MiniMax M2 cache-read set to 0.6/MTok to match OpenRouter's current rates.
- **Branding: pi→hamr in error/crash/log paths.** `PI_DEBUG_REDRAW` → `HAMR_DEBUG_REDRAW`, crash logs moved from `.pi/` to `.hamr/`, uncaught-exception message corrected from "pi exiting" to "hamr exiting".

## [0.6.3] - 2026-06-24

### Added

- **`node:sqlite` adapter for FTS5 memory.** A pure-JS fallback for `better-sqlite3` backed by Node's built-in `node:sqlite` (stable since Node 24). Provides FTS5-enabled `Database` when the native better-sqlite3 addon has no prebuilt binary for the running Node ABI — no native compilation needed. Transparent at runtime; `better-sqlite3` is still preferred when available.
- **Subagent artifact contract.** `TaskItem` now accepts an optional `artifact` field — a path to an output file the worker must produce. After completion, the runner validates the file exists and is non-empty; failures are treated as worker errors.
- **Empty-output detection for subagent workers.** Workers that produce only thinking events (no final assistant text) are now treated as failed. The aggregate result is marked `isError: true` when all completed workers produced empty output — previously these were silently counted as succeeded.
- **Strict CWD sandboxing in PathGuard.** Opt-in `strictCwd` option confines all file read/write operations to the working directory. Off by default (agents legitimately operate across repo boundaries); the denylist-based PathGuard remains the primary security boundary.
- **Provider proxy exclusion.** Known provider hosts are auto-added to `NO_PROXY` at startup, so credentials and CF-Access headers are never forwarded to a configured HTTP proxy. Provider traffic always uses direct connections.
- **Extended security denylist.** Added 9 new paths to the read/write denylist: `.bash_history`, `.zsh_history`, `.zhistory`, `.mysql_history`, `.psql_history`, `.python_history`, `.node_repl_history`, `.npm/_cacache`, `.docker/config.json`, `.kube/config`.
- **Inline high-severity validation warnings in subagent results.** High-severity output warnings are now surfaced directly in the aggregate card (up to 3 shown inline, with a count of remaining), instead of being hidden behind the Ctrl+O expand.
- **18 new text models in the registry.** OpenRouter: `sakana/fugu-ultra`, `alibaba/qwen3-vl-235b-a22b-instruct`, `alibaba/qwen3-vl-instruct`, `amazon/nova-2-lite`, `amazon/nova-lite`, `amazon/nova-micro`, `amazon/nova-pro`, `arcee-ai/trinity-mini`, `bytedance/seed-1.8`, `interfaze/interfaze-beta`, `kwaipilot/kat-coder-pro-v1`, `meituan/longcat-flash-thinking-2601`, `mistral/magistral-medium`, `mistral/magistral-small`, `mistral/ministral-14b`, `mistral/mistral-large-3`, `nvidia/nemotron-3-nano-30b-a3b`, `openai/gpt-3.5-turbo`, `zai/glm-5.2-fast`.
- **3 new image models.** OpenRouter: `openai/gpt-image-1`, `openai/gpt-image-1-mini`, `openai/gpt-image-2`.

### Fixed

- **`maxTokens` default for reasoning models: 65536 (was 16384).** Reasoning models now get a higher default max output tokens (65K vs 16K), clamped to the model's context window. Applied in both `startup-config` (model registry layer) and `sdk` (child-process layer).
- **Zero-usage context estimation for relay/local providers.** Providers that return all-zero usage in streaming chunks now fall back to char-count heuristics for context estimation, so the footer shows a useful percentage instead of 0%.
- **Full event JSON persisted to subagent disk logs.** `events.ndjson` now stores complete event payloads for forensic replay. Only the in-memory `recentEvents` ring buffer (shown in the UI) is truncated to 256 chars. Disk writes use `0o600` permissions.
- **Subagent result files write full output.** Worker final output is now written to disk in full — previously it was capped at `OUTPUT_TAIL_BYTES` alongside the in-memory preview.
- **Child session path matches parent encoding.** Spawn points now use the shared `getDefaultSessionDirPath` encoder, honoring any configured agent directory so the session tree can cross-reference child sessions.
- **Post-update version check ran `node --version` instead of `hamr --version`.** `checkPostUpdateVersion` spawned `process.execPath` (Node.js) with `--version`, returning Node's version string (`v26.0.0`) instead of hamr's version, which broke the confirmation message and internal version detection. Now passes `process.argv[1]` (the CLI entrypoint) so it properly runs `hamr --version`.

### Changed

- **Model pricing and capabilities updated.** MiniMax M2, Claude Opus 4.7, DeepSeek V4 Pro, DeepSeek V4 Flash, Xiaomi Mimo, and GLM-5 reflect current OpenRouter pricing and revised max output tokens.
- **Supply-chain hardening.** `.npmrc` enforces `save-exact=true` and `min-release-age=2`. Pre-commit hook blocks accidental `package-lock.json` commits unless `HAMR_ALLOW_LOCKFILE_CHANGE=1` is set.

## [0.6.1] - 2026-06-23

### Added

- **Custom endpoint form in /login for local providers.** The login page now offers a form to configure custom/self-hosted inference endpoints (LM Studio, llama.cpp, Ollama, vLLM, Custom) without going through the full /login flow. Saves to `~/.hamr/agent/models.json`, auto-discovers models, and switches provider immediately — no restart needed.
- **Agent-layer relay auth resolution.** Providers now resolve credentials from `models.json`/`auth.json` in the agent layer, falling back to `hamr.toml` and then local defaults. Relay endpoints whose credentials live in the agent layer are now probed and registered with the correct auth instead of falling back to an unauthenticated placeholder.

### Fixed

- **Unified endpoint discovery auth with chat path.** Endpoint discovery now uses the same auth flow as the chat path, ensuring credentials are applied consistently whether the provider is discovered via /login or registered dynamically.
- **Stale model-loading indicator cleared on response.** The model-loading status text is now cleared when an assistant response arrives, so the indicator no longer sticks after the model has responded.

## [0.6.0] - 2026-06-22

### Added

- **Custom / self-hosted endpoint configuration in TUI.** `/login` now offers a third option "Use a custom/self-hosted endpoint" alongside subscriptions and API keys. Opens a form with presets (LM Studio, llama.cpp, Ollama, vLLM, Custom), URL/API type/key fields, and dynamic header editing. On save, writes to `~/.hamr/agent/models.json`, auto-discovers models from the endpoint via `GET /models`, and switches to the new provider immediately — no restart needed.
- **24 new models in the registry.** HuggingFace: MiniMax-M2, MiniMax-M3, Qwen3-235B-A22B, Qwen3-32B, Qwen3-Coder-30B-A3B, Qwen3.5-122B-A10B, Qwen3.5-27B, Qwen3.5-35B-A3B, Qwen3.5-9B, Qwen3.6-35B-A3B, DeepSeek-R1, DeepSeek-V4-Flash, Gemma-4-26B-A4B-it, Gemma-4-31B-it, Llama-3.3-70B-Instruct, Kimi-K2.7-Code, Step-3.5-Flash, GLM-4.5, GLM-4.5-Air, GLM-4.5V, GLM-4.6, GLM-5.2. OpenRouter: `z-ai/glm-5v-turbo`, `sakana/fugu-ultra` (Anthropic Messages protocol). Removed `nex-agi/nex-n2-pro:free`.
- **Dynamic model IDs through the type system.** `ModelApi`, `getModel()`, and `getModels()` now accept arbitrary `string` model IDs instead of being locked to literal keys from the registry — runtime-discovered models (from custom endpoints) no longer cause type errors.

### Changed

- **Docs rewritten around `models.json`** rather than TOML. All provider, relay, and configuration docs now show JSON examples for custom endpoints. Removed custom dark-theme VitePress layout (HamrLanding, nav logo, 110 lines of CSS) in favor of stock VitePress.

### Fixed

- **Self-update tests create dummy `dist/cli.js`** before spawning the CLI, preventing `ENOENT` crashes from `checkPostUpdateVersion`.
- **Dist files rebuilt** to catch up with v0.5.2 source changes that weren't compiled before tagging (package-manager-cli `checkPostUpdateVersion`, better-sqlite3 `failedPaths` caching + `:memory:` probe, hamr theme `shadedSurfaces`/`gaplessCards`).

## [0.5.2] - 2026-06-21

### Fixed

- **"Run hamr update" notification now links to the correct GitHub release changelog** instead of `hamr.dev/changelog`.
- **Self-update now verifies the new version actually installed.** After `hamr update` completes, it spawns the new binary to confirm the version changed. If the old version is still running (Homebrew prefix, nvm path edge cases), it prints a clear warning with the `npm install -g` fallback command.

## [0.5.1] - 2026-06-21

### Fixed

- **Fatal: better-sqlite3 native binding crash on Node 25.** `require("better-sqlite3")` succeeded but `new Database()` threw when the native `.node` binary was missing (no prebuild for Node 25 / ABI v141 on Linux). The loader now validates the binding with a `:memory:` probe. Init failures are cached so they don't retry and re-log on every turn.

## [0.5.0] - 2026-06-21

### Breaking

- **Package renamed: `@hamr/coding-agent` → `@skaft/hamr`.** The published CLI package is now `@skaft/hamr`. Existing `npm install -g @hamr/coding-agent` installations must be migrated: `npm uninstall -g @hamr/coding-agent && npm install -g @skaft/hamr`. The old `@hamr/coding-agent` import surface for extensions remains supported as a virtual module alias — existing extensions continue to work without changes. Self-update (`hamr --update`) handles the migration automatically (single in-place install, no uninstall/install dance). Internal workspace packages (`@hamr/agent`, `@hamr/ai`, `@hamr/tui`) keep their `@hamr/` scope.
- **Skills no longer bundled as `skills/*.md`.** Built-in skills (`using-hamr`, `frontend-design`, plus all askr skills) are now bundled from `dist/askr/skills` at build time. The `skills/` directory is removed from the npm package and from the `hamr.skills` manifest. Duplicate detection prevents bundling askr when the user has already installed it via pi packages, so the user-installed version always wins.
- **`supports_thinking = false` no longer disables thinking on a built-in reasoning model.** Model *capability* now comes from the model registry, not from `.hamr.toml`. A config entry can still enable thinking on a model the registry doesn't know, but it can no longer strip thinking from a model the registry says reasons. If you previously used `supports_thinking = false` to turn thinking *off* on a capable model, that no longer works — use `default_thinking = "off"` on the model, a narrowed `thinking_levels` list, or the global `defaultThinkingLevel: "off"` in `settings.json` instead. No toml migration is required; the fix applies on upgrade and stuck-off thinking self-heals.

### Added

- **Subagents: parallel/chain/stages with child-process workers.** The `delegate_subagents` tool was rewritten from fork-based serial execution to a full child-process executor. Workers spawn as isolated `hamr --mode json -p` processes. Three modes: `tasks` (parallel batch with bounded concurrency), `chain` (serial with `{previous}` placeholder), and `stages` (sequential stages, each parallel or chain internally). Full JSONL disk-persisted logs (`.hamr/subagents/runs/<runId>/`); only bounded recent events and output tails are kept in memory. Configurable via env vars: `HAMR_SUBAGENT_MAX_TASKS` (default 64), `HAMR_SUBAGENT_MAX_CONCURRENCY` (default 64), `HAMR_SUBAGENT_MAX_LOCAL_CONCURRENCY` (default 1 for relay/local). Live status widget above the editor shows running/queued/done/failed counts.
- **Per-worker model and tools overrides.** `TaskItem` accepts optional `model` (per-worker model selection, e.g. `claude-haiku-4-5` for cheap subtasks) and `tools` (restrict the tool set per worker, e.g. `["read", "grep"]` for read-only recon workers). Both are passed as `--model` and `--tools` CLI args to the child `hamr` process.
- **Subagent result rendering with Markdown and formatted tool calls.** Collapsed worker cards show a one-line summary with tool name. Expanded results render Markdown output and display tool calls with human-readable formatting (e.g. `$ npm test`, `read src/file.ts:42-99`, `write config.json (34 lines)`). An aggregated total line shows combined token usage and cost across all workers.
- **Provider-agnostic model loading events.** Both SSE comment-based (`:relay loading model=...`) and JSON-based (`{"event":"loading",...}`) cold-start indicators are now surfaced in the TUI status bar and emitted as `model_loading` agent events. No longer restricted to relay — any provider that emits loading events gets shown.
- **Adaptive card backgrounds.** Theme cards now compute blend-adaptive backgrounds from the detected terminal background color via HSL color-space blending. Cards lift slightly lighter on dark terminals and sink slightly darker on light terminals, respecting the terminal's native theme. Controlled by `adaptiveBackground` in theme JSON (default true for built-in themes).
- **Kitty keyboard protocol key-release support.** `SettingsList` now accepts `wantsKeyRelease` so Escape works when the terminal only sends release events (Kitty flag 2). The global interrupt listener filters out key-release events so they don't steal focus from components that handle them. Added `isKeyRelease` and `isKeyRepeat` utilities to `@hamr/tui`.

### Changed

- **Dependencies restructured for the `@skaft/hamr` fat package.** Internal `@hamr/agent`, `@hamr/ai`, and `@hamr/tui` are now pinned (`0.79.7`), bundled via `bundleDependencies`, and have their transitive dependencies hoisted to the top-level `dependencies`. `protobufjs` is bundled with its postinstall script stripped to work around an npm@11 lifecycle bug on global installs. Consumers no longer need to install anything beyond `@skaft/hamr`.
- **Adjacent user messages are no longer merged.** The OpenAI-compatible provider previously merged consecutive user messages into a single block to satisfy strict relay chat templates. This is removed because merged messages broke Anthropic's longest-prefix prompt cache (the merged content changed every turn). Adjacent user messages now pass through as separate turns.
- **Prompt cache key always sent for relay.** Previously the `prompt_cache_key` was gated on `sessionId` being present for relay. It is now always sent for relay providers, matching the `session_id` header behavior.
- **Skills index always shown in system prompt.** Previously the skills section was gated on whether the `read` tool was active. Skills are now always included (progressive disclosure — the model loads full SKILL.md on demand via `read`). The skill discovery guideline was tightened to `"Discover and load it via \`ls\`/\`read\`"`.
- **Footer refactored.** Context usage line is always shown (not conditionally hidden when context is low). Cost line always shown when there's accumulated spend or an active OAuth subscription. Token formatting improved: 4-digit values now show one decimal (`12.3K`), 7-digit values show one decimal (`1.2M`). Removed dead `autoCompactEnabled` plumbing.
- **Context breakdown (`/context`) display fixed.** The 25-slot icon grid now fills based on absolute token count (4K tokens per slot) instead of percentage, so the bar is meaningful at any context size. Overflow is shown with a `+` marker. Skills now show as 0 tokens since they're loaded on demand. The display handles edge cases (`0` context window, null tokens) gracefully.
- **Hamr Browser close hardened.** `browser.close()` is now called after `context.close()` to force-kill the underlying Chromium process for `launchPersistentContext`.
- **Card layout refreshed.** Default card indents reduced (`bodyIndent: 1`, `toolResultIndent: 1`). Cards are no longer gapless by default (`gaplessCards: false`). Theme colors updated across dark, hamr, kawaii, light, and pinkOut themes.
- **Build: askr bundled at build time.** A `bundle-askr` script fetches the latest askr release from GitHub and copies skills into `dist/askr/skills/` during `npm run build`. Removes the stale vendored `skills/` directory.
- **Pipeline keywords changed to uppercase K/M.** Token formatting everywhere now uses `K` and `M` instead of `k` and `m` for consistency with standard notation.
- **Splash screen safety guards.** The splash no longer blind-clears the chat container when messages are present. `splashRendered` is reset on session state changes to prevent stale state.

### Fixed

- **Prompt caching efficiency restored to ~99% (was 25-30%).** Three fixes targeting Anthropic's longest-prefix cache model:
  - Memory context messages are now **appended** (not prepended) to the conversation, preserving the stable prefix for cache hits.
  - Memory context injection is **throttled** (every 5 turns for cloud, every 2 for local) and **deduplicated** via content hashing — no longer injected on every turn.
  - Cloud providers (Anthropic, OpenAI) now **skip FTS5 context injection entirely** unless a survival manifest exists; they use proper LLM compaction instead.
- **Relay providers now emit Anthropic-style `cache_control` markers** on system prompts, tools, and conversation messages. Session-affinity headers (`x-session-affinity`, `session_id`, `x-client-request-id`) are sent for cache-replica routing. Long cache retention (`ttl: 1h`) is enabled. Markers are harmless for backends without caching support.
- **Relay configured models inherit built-in cost and context.** When a `.hamr.toml` model entry shadows a known built-in model id (same provider), it now inherits pricing and context limits from the built-in rather than zeroing them — so e.g. adding an API key for the built-in `deepseek` provider keeps its real cost data.
- **Configured models inherit built-in thinking capability.** A `.hamr.toml` entry that shadows a built-in reasoning model now inherits the registry's `reasoning` flag and `thinkingLevelMap` instead of being silently forced off by a missing or `false` `supports_thinking`. This was hiding the thinking selector for models the registry knows reason (e.g. `deepseek-v4-flash` with a probe-defaulted `supports_thinking = false`), while `deepseek-v4-pro` worked — same model data, different config line. Capability is now owned by the model registry; config may only widen it, never strip it. See the Breaking note above for the one behavior that flips.
- **Kitty key-release interrupt fix.** The global interrupt listener now filters out key-release events so the focused component (e.g. SettingsList) receives the press event without interference.
- **Subagents: parallel mode forced sequential due to missing provider-aware concurrency.** The concurrency clamp previously used a single hardcoded cap regardless of provider type. Concurrency is now provider-aware via `isCloudProvider()` — cloud providers get the higher `ENV_MAX_CONCURRENCY` (default 64), while relay/local providers are capped at `ENV_MAX_LOCAL_CONCURRENCY` (default 1) to avoid rate-limiting. Users on rate-limited local providers should set `HAMR_SUBAGENT_MAX_LOCAL_CONCURRENCY=1` explicitly if auto-detection misclassifies their provider.
- **Subagents: O(W) → O(1) status counters.** `updateRunCounts` scanned all workers on every status change, causing O(W²) churn when many workers completed near-simultaneously. Replaced with atomic counter increments (`transition()`).
- **Subagents: synchronous per-event disk writes replaced with batch flush.** `appendFileSync` was called for every event from every child (N×E sync writes). Events are now collected in the in-memory ring buffer and flushed to disk once per worker at completion, one sync write per worker lifetime. Note: the parent's `.events.ndjson` now stores at most the last 40 events per worker (truncated summary format); the child process's own session log remains the authoritative full record.
- **Subagents: chain mode abort listener leak.** `runWorkerChildProcess` registered an `abort` listener on the parent signal for each worker but never removed it when the worker exited normally. Over long sessions with many swarm calls, stale `killProc` closures accumulated on the parent `AbortSignal`. Fixed: the listener is removed via `proc.on("close")`.
- **Subagents: chain mode now uses per-step AbortController.** Previously the parent agent's streaming `AbortSignal` was passed directly to every chain step's child process. Any internal lifecycle event (compaction, auto-retry, session dispose) that aborted the parent signal would kill the current chain step and cascade-abort all remaining steps. Each step now gets its own `AbortController` that forwards the parent signal (user escape still works) but cleans up its listener after each step. A `console.error` diagnostic is logged if the parent signal fires mid-step.
- **Subagents: aborted workers no longer counted as succeeded.** The summary builder excluded aborted results from the error count, causing an all-aborted run to report "N/N succeeded." Aborted workers are now counted separately in the summary (e.g. "0/8 succeeded, 8 aborted").
- **Security: path-confinement for file tools.** Read, write, and edit tools now block access to credential and persistence locations (SSH keys, `~/.aws`, `~/.bashrc`, `.git/hooks`, auth tokens). Configurable; on by default.
- **Security: proxy URL validation.** HTTP proxy settings are now validated and warned before application. Invalid URLs and non-http(s) schemes are rejected.
- **Security: subagent temp files secured.** Child config and output files now use restrictive permissions (`0o600`) with parent-exit cleanup to prevent credential leaks from crashed sessions.
- **Security: subagent process-tree cleanup.** Workers are detached and tracked; killing the parent kills all children. Budget slots are refunded on early failure.
- **Security: extension exec timeout.** Extension-spawned commands now have a default 10-minute timeout to prevent hung subprocesses from blocking the agent.
- **Security: auth-storage lock busy-wait fixed.** Replaced with `Atomics.wait`-based sleep to avoid CPU spin during token-refresh contention.
- **Hardened `.gitignore`** against accidental config credential commits.
- **Fixed: adaptive card backgrounds rendered black when terminal bg could not be detected.** The COLORFGBG fallback for dark terminals now uses `#1a1a1a` instead of pure black, and the elevation computation applies a lightness floor so cards are always distinguishable from the terminal background.
- **Separate lifecycle and tool abort signals.** The agent now distinguishes between lifecycle aborts (compaction, auto-retry — stop LLM streaming only) and tool aborts (user escape, session dispose — also kill running tools). Subagents and other long-running tools are no longer spuriously killed by internal lifecycle events.

## [0.4.0] - 2026-06-19

### Breaking

- **License: Apache 2.0 → MIT.** Relicensed under the MIT License. Added `NOTICE.md` with full third-party attribution for pi (Mario Zechner / `@earendil-works/pi`) and `sexy-tui-rs`. Added acknowledgments section to `README.md`.
- **Anthropic model IDs renamed: `frontier-*` → `claude-*`.** All Anthropic model identifiers, display names, and config keys have been renamed. `frontier-sonnet-4-20250514` → `claude-sonnet-4-6` / `claude-sonnet-4-5-20250929`, `frontier-3-5-haiku` → `claude-haiku-4-5`, `frontier-3-opus` → `claude-opus-4-7`. The `coreVisualProfile` value `"frontier"` is now `"claude"`. Pricing keys, provider presets, image-utils vision regex, and all tests are updated.
- **Orchestration: mutating plans always serialized.** Previously parallel mode was only downgraded to sequential when sub-task file scopes overlapped. Now any plan containing mutating tasks (non-`none` verification) is always serialized, since scope hints are advisory and cannot prevent race conditions on the shared working tree. Read-only recon plans keep their parallelism.

### Added

- **`.env` auto-loading (`src/config/load-dotenv.ts`).** On startup, Hamr loads `~/.env`, `~/workspace/.env`, and `<cwd>/.env` into `process.env` (idempotent, existing vars never overwritten). Respects `HAMR_SKIP_DOTENV=1`. Parses shell-style `export KEY=VALUE` lines. Wired into both `cli.ts` and `bootstrap.ts` for early availability.
- **New Claude models.** Added `claude-sonnet-4-6`, `claude-haiku-4-5`, and `claude-opus-4-7` to default model configs with full capability descriptors (context window, thinking levels, vision support).
- **DeepSeek V4 Flash context window corrected to 1M.** Previously hardcoded at 128K in both provider presets and default model config.
- **3-tier model glyph system in TUI.** Each model family now carries a Nerd Font icon (Material Design Icons PUA codepoints), a Unicode geometric fallback, and an ASCII letter fallback. `terminalGlyph()` selects the appropriate tier based on `TerminalCaps`, preventing tofu boxes on terminals without patched fonts.
- **Explicit thinking-disable signals.** When `thinkingLevel` is `off`, the LLM client now sends explicit opt-out knobs (`thinking: { type: 'disabled' }`, `enable_thinking: false`, `chat_template_kwargs: { enable_thinking: false }`) rather than silence. It never sends `reasoning_effort: "none"` because strict providers reject it. Local chat templates (Qwen3, Cohere North/Command) default reasoning ON, so silence was not "off."
- **Synthetic tool results on abort/limit/hook-block.** When a turn is aborted, max tool calls are exceeded, or a `pre_tool_use` hook blocks execution, the session now appends a synthetic error tool result to the conversation. This preserves strict tool-call/result pairing and prevents desynchronization on recovery.
- **Turn ID tracking and proactive cancellation.** The TUI now assigns incrementing turn IDs and tracks `cancelledTurnId`. A 5-minute safety timeout auto-aborts stuck turns. Cancellation via Ctrl+C or Escape sets the cancelled turn ID without immediately resetting `processing`, so the in-flight turn can settle gracefully before re-enabling the editor.
- **Context bar scaling.** The context-usage bar width now scales with the context window (1 char per 25K tokens, min 20, max 60). Non-zero categories get at least 1 char so tiny usage doesn't disappear. Compact token format (e.g. `1.0M`) on narrow (<80 col) terminals.
- **Provider-agnostic loading events.** Cold-start loading indicators in the TUI are no longer restricted to the Relay provider — any provider that emits loading SSE events gets shown.
- **Pi-style assistant streaming on `sexy-tui-rs`.** Live model output now updates one assistant stream card containing thinking + visible text, instead of emitting separate accumulated thinking cards.
- **"Hamr" default theme.** First-time setup and fresh installs now default to the "hamr" theme. Terminal background detection is no longer used for theme selection; the detected appearance is shown for informational purposes only. `getDefaultTheme()` returns `"hamr"` instead of `"dark"`.

### Changed

- **TUI alternate screen defaults to off (primary buffer).** The `alternateScreen` default changed from `true` to `false` — the primary buffer gives users native terminal scrollback and text selection (select + shift-click). Set `alternateScreen = true` in `.hamr.toml` for the alternate-screen experience. SGR mouse tracking stays default off (opt-in via `enableMouse`).
- **TUI visible-width measurement delegates to native renderer.** `EventFeed.visibleWidth()` and `truncateLine()` now use `sexy-tui-rs`'s `visibleWidth` and `truncateToWidth` directly instead of a local heuristic. The old heuristic miscounted symbols like ⚠ → ℹ ✗ … (2 cols vs the renderer's 1), causing width-guard crashes and uneven card backgrounds. The mock was updated with matching test-only implementations.
- **TUI render budget: newest-first, oldest-drop.** When the line budget (5,000 lines) is exhausted, the oldest events are dropped instead of cutting off the newest messages. A warning banner is shown at the top of the feed with the count of hidden events.
- **TUI diff rendering fixes.** Unified-diff file headers (`--- a/file`, `+++ b/file`, `diff --git`, `index`) are now detected and rendered as muted preamble lines instead of being misrouted through the +/- gutter path (which incorrectly rendered them as removed/added content with bogus line numbers).
- **TUI crash recovery hardened.** Separate `uncaughtException` and `unhandledRejection` handlers clean up the terminal (raw escape sequence fallback if `terminal.stop()` throws) before exiting. Signal and resize handlers are properly unregistered on shutdown.
- **TUI render loop adapts to cmux mode.** The status-bar refresh interval is 250ms under cmux (vs 80ms normally) to reduce overhead on multiplexed connections.
- **Identical-read loop detection moved after result appending.** The check now runs after `appendToolResult()` so recovery requests preserve strict tool-call/result pairing.
- **Session store signals exit with correct codes.** `SIGINT` → `exit(130)`, `SIGTERM` → `exit(143)` (128 + signal number).
- **HolographicMemory caches empty suggested-terms arrays.** Previously `null` was cached for empty results, causing re-queries. Now empty arrays are cached to avoid redundant computation.
- **Chat command resolves provider name from effective config.** The status bar and model picker now use the resolved provider name from `effectiveSettingsConfig` rather than the metadata display name, ensuring provider renames are reflected everywhere.
- **TUI card right-padding for unshaded cards.** Cards without background shading now get right-side padding for visual parity with shaded cards.
- **`tool_result` artifact type handled in EventFeed.** Cards with `artifact.type === 'tool_result'` now render correctly alongside the existing `text` type, with combined summary/output display.
- **Session replay preserves streamed assistant state.** JSONL sessions now store assistant stream deltas so resume can reconstruct thinking/text blocks and model-visible history.
- **All themes switched to gapless card layout, no thought headings.** dark, hamr, kawaii, light, and pinkOut themes unified on `gaplessCards: true`, `showThoughtHeading: false`, and reduced heading/body/tool indentation. Trailing blank lines removed from all theme JSONs. `$schema` references changed from remote URLs to local `./theme-schema.json` for dark and light.
- **TUI: trailing spacer after every content block.** Dynamic borders, warnings, errors, changelog/version/hotkeys/debug-log displays, Armin/Daxnuts components, and login-dialog instructions/code now all end with a trailing `Spacer(1)` for consistent visual breathing room.
- **TUI: tool cards always respect cardPadY.** `ToolExecutionComponent` no longer returns `0` padding in gapless mode — it always uses `theme.cards.cardPadY`.
- **TUI: read tool render padding explicit.** Call and result `Text` components in the read tool definition use explicit `0` padding instead of reading `theme.cards.cardPadY`.
- **System prompt tightened.** Agent introduction reworded to "Diligent, laconic coding agent." Tool usage and rules text streamlined for brevity.
- **Skills prompt tightened.** The skills section now reads: "Use skills only when the user's task clearly requires one. Do not load skills for greetings or general chat."

### Fixed

- **TUI card surfaces unified with theme option.** Prompt, response, and tool cards now all respect `theme.cards.shadedSurfaces`, so tool calls can match the same card styling as messages when surfaces are enabled.
- **Tool-call cards now match message cards more closely.** `ToolExecutionComponent` now uses the same shaded card treatment as prompt/response cards, with self-rendered tool output flush to the card edge and branded headings retained.
- **`isVisionCapableModel` regex rewritten for Claude naming.** The Anthropic regex now matches `claude-*` patterns (`opus`, `sonnet`, `haiku`, `fable`, `mythos`) instead of the old `frontier-*` patterns.
- **Empty `command` field on verification events no longer crashes `classifyAgentEvent`.** Guard clauses added for `verification_started`, `verification_skipped`, and related events where `event.command` may be undefined.
- **`OrchestrationManager`: overlapping-scope check removed.** The `hasOverlappingFileScopes` guard was both insufficient (advisory hints only) and misleading. Replaced with the blanket rule: any mutating plan is sequential.
- **Thinking card accumulation fixed.** Streaming providers that send cumulative partials are normalized to true deltas; the TUI no longer maintains a separate append-only thinking buffer.
- **Read-only report loops bounded.** Report-back tasks now nudge after repeated inspection-only steps and stop instead of looping through broad `git diff`/`read` probes forever.
- **Abort accounting improved.** Interrupted turns now preserve best-effort step/tool counts instead of reporting zero work.

### Removed

- **Apache 2.0 license text.** Replaced with MIT license including pi/sexy-tui-rs attribution notice.
- **Legacy `frontier-*` model IDs.** All references removed from default models, provider presets, pricing tables, tests, docs, and regex patterns.
- **Deleted docs pages.** `AiCore.vue`, `CoreTranscriptHero.vue`, `RuntimePanel.vue`, `TerminalPreview.vue`, `runtime-core.ts`, `SUPER_EDITION_BRANCH.md`, `super-boundary.md`, `agent-loop-tui-crash-review.md`, and 5 files under `docs/superpowers/` (specs and implementation plans).
- **Built-in askr skill package.** `BUILTIN_SKILL_PACKAGES` in `main.ts` is now empty — askr skills are no longer auto-loaded on startup.


## [0.3.0]

### Breaking

- **Rebrand: synax → hamr.** All public-facing names, file paths, config keys, docs, scripts, and source identifiers have been renamed from `synax` to `hamr`. This includes `.synax.toml.example` → `.hamr.toml.example`, `benchmarks/synax-auto-research/` → `benchmarks/hamr-auto-research/`, `SynaxRuntime` → `HamrRuntime`, and all doc references. The package name and binary remain unchanged.
- **TUI rewrite.** The old TUI module (`src/tui/interactive-tui.ts`, `src/tui/autocomplete.ts`, `src/tui/input.ts`, `src/tui/key-handlers.ts`, `src/tui/model-palette.ts`, `src/tui/opentui-artifact-renderer.ts`, `src/tui/opentui-render-scheduler.ts`, `src/tui/text-utils.ts`, `src/tui/theme.ts`, `src/tui/token-stream.ts`, `src/tui/tui-constants.ts`) has been removed and replaced with a new architecture (`src/tui/bootstrap.ts`, `src/tui/hamr-tui.ts`, `src/tui/components/`, `src/tui/theme/`). Tests for the old TUI (`interactive-tui.test.ts`, `opentui-render-scheduler.test.ts`, `tui-usability.test.ts`) have been deleted.

### Added

- **Dynamic model discovery from `/v1/models`.** All providers now probe `/v1/models` on startup and replace static defaults with the server's authoritative model list.
- **Model-family prompt box coloring.** The editor border color is derived from the active model's family accent and brightened/dimmed by thinking level.
- **Automatic orchestration policy.** Cloud providers auto-dispatch parallel agents for decomposable tasks; local providers fall back to inline execution.
- **Model picker context window display.** Each model in the picker shows its context window size (e.g. `131K ctx`).
- **`paste_context_range` tool.** Slices a range from the last user message using line numbers, anchor text, or byte offsets and writes it to a temp file.
- **Image paste/drag support in TUI.** Pasting or dragging image files into the prompt encodes them as multimodal content blocks.
- **Multiple named themes.** 8 new TUI palettes: gruvbox, kanagawa, catppuccin, nord, rose-pine, tokyo-night, pink, and dracula.
- **Runtime environment context injected into skill messages.** Model now receives repo path, home directory, username, and platform.
- **Session resume rebuilds transcript from persisted event log.** Resumed sessions show prior conversation in the TUI feed.

### Changed

- **Toolchain: tsc → tsgo, eslint+prettier → biome, ts-jest → @swc/jest.** TypeScript 7 native Go compiler (`tsgo`) replaces `tsc` (~10× faster). Biome replaces ESLint and Prettier. SWC replaces ts-jest.
- **Brighter default theme.** Darkened backgrounds (`#15171a`), lightened foregrounds (`#e0e4eb`), more saturated accent colors. Purple is now the default accent.
- **Splash screen redesign.** Styled HAMR wordmark with model-palette accent colors, decorative box frame, tagline, and version line.
- **Thinking cards finalized as "Thought".** Retitled and dimmed on completion; total elapsed time shown once in the header.
- **Settings/resume overlay lines update in place.** No full tree rebuild on navigation within overlays.
- **CI/CD: deploy to Cloudflare Pages.** Docs deployment uses `cloudflare/wrangler-action`.

### Fixed

- **Thinking buffer leaked across turns.** Buffer now resets on `user_message` and `task_started` events.
- **Scroll viewport pinned to top instead of bottom.** Fixed inverted scroll offset math.
- **`?` key could not be typed into the prompt.** Global input listener no longer consumes `?`.
- **Splash erased on settings changes.** Settings pickers update splash inline when visible.
- **Orchestration `shouldOrchestrate` compared to wrong string.** Fixed `'orchestrated'` → `'orchestrate'`.
- **Session ID collision defenses.** Better error catching and prevention in `EventStore`, `SessionFactory`, and `generatePersistentSessionId`.
- **Robust bracketed paste handling.** Position-aware detection, keypress suppression during paste, multi-byte UTF-8 support.
- **Prompt box overflow with word-wrap height simulation.** Matches OpenTUI's `wrapMode: 'word'` behavior.

### Removed

- **Old TUI modules.** Deleted `src/tui/interactive-tui.ts`, `src/tui/autocomplete.ts`, `src/tui/input.ts`, `src/tui/key-handlers.ts`, `src/tui/model-palette.ts`, `src/tui/opentui-artifact-renderer.ts`, `src/tui/opentui-render-scheduler.ts`, `src/tui/text-utils.ts`, `src/tui/theme.ts`, `src/tui/token-stream.ts`, `src/tui/tui-constants.ts` and their tests.
- **Static model default for relay.** Replaced by probed model list at startup.
- **Per-line thinking duration counters.** Only total elapsed time shown in header.
- **Extra vertical padding around thinking cards.** Redundant with inter-class spacing.


## [0.3.0-beta]

### Breaking

- **Read results are now budgeted and truncated.** Previously "dogfooding mode" passed through all read output untruncated. Read results are now subject to a per-read token cap (with continuation guidance via `startLine`) and a per-turn cumulative cap, after which further reads are refused with a recoverable policy error. This keeps context from being swamped by large reads.
- **`maxToolCalls` and `maxModelSteps` are now enforced.** Previously both were set to `Number.MAX_SAFE_INTEGER` (effectively unlimited). Defaults are now `maxToolCalls=192`, `maxModelSteps=64`. Runaway loops now hit a hard stop with `budget_exhausted`.
- **Bash is now enabled by default.** Previously disabled-by-default. The docs and `[tools.bash]` behavior are updated accordingly. Disable via `[tools.bash] enabled = false`.
- **Per-turn read budget cap removed.** The cumulative per-turn token cap on read results (`maxTotalReadResultTokensPerTurn`) is now 0 (unlimited). The context window itself, compaction, and subagent handoff serve as the natural budget. Hard-capping reads mid-investigation was amputating the model: once exhausted, every read returned an error and the model could not gather any new information.
- **Identical-read loop detection is now a hard stop.** Previously the read handler injected soft nudges on repeated identical reads. Now `Session` tracks identical-read counts (keyed by path + line range) and terminates the turn with `tool_error` after 5 consecutive identical reads. Dogfooding mode (`HAMR_DOGFOOD`) disables the limit. Different line ranges on the same file are NOT treated as identical.

### Added

- **Dynamic model discovery from `/v1/models`.** All providers now probe `/v1/models` on startup and replace static defaults with the server's authoritative model list. The relay server's `RELAY_MODEL_MAP` is the single source of truth — add a model there and it appears in the TUI model picker on next launch. Server-provided `display_name`, `supports_thinking`, and `thinking_levels` fields are consumed directly. Falls back to static defaults on probe failure.
- **Model-family prompt box coloring.** The editor border color is derived from the active model's family accent (purple for Qwen, blue for DeepSeek, orange for Claude/Mistral, white for OpenAI/GLM/Kimi, etc.) and brightened/dimmed by thinking level. Higher thinking → more vivid prompt box.
- **Automatic orchestration policy.** Cloud providers auto-dispatch parallel agents for decomposable tasks. Local providers fall back to inline execution with a suggestion to use "use parallel agents" explicitly. Explicit delegation always honors the requested mode.
- **Model picker context window display.** Each model in the picker shows its context window size (e.g. `131K ctx`) alongside the display name.

### Changed

- **Toolchain: tsc → tsgo, eslint+prettier → biome, ts-jest → @swc/jest.** TypeScript 7 native Go compiler (`tsgo`) replaces `tsc` (~10× faster). Biome replaces ESLint and Prettier as a single unified linter/formatter. SWC replaces ts-jest for Jest transpilation.
- **Brighter default theme.** Darkened backgrounds (`#15171a`), lightened foregrounds (`#e0e4eb`), more saturated accent colors. Purple is now the default accent.
- **Prompt box colored by model family + thinking level.** The editor border uses the model's family color (detected from model name), brightened progressively as thinking level increases (off→30%, low→45%, medium→65%, high→85%, xhigh→100%).
- **Model accent colors expanded.** Claude/frontier/opus/sonnet/haiku/fable/mythos → dark orange. Mistral/devstral/codestral → light orange. DeepSeek → dark blue. Gemma → mid blue. Gemini → light blue. Qwen → purple. MiniMax → red. OpenAI/GLM/Kimi → white. Default → blue.
- **Splash persists until user prompt.** Internal events (thinking, status) and settings-change notes no longer hide the splash screen. The splash is only replaced when the user sends their first message.
- **Settings changes update splash inline.** Changing model or thinking level while the splash is visible updates it in place rather than adding a note card.
- **Thinking cards finalized as "Thought".** When thinking completes, the card retitles from "THINKING" to "THOUGHT" and dims. Total elapsed time is shown once in the header during live thinking.
- **Response headers use semantic colors.** Successful responses (`◇ RESPONSE`) are green, errors (`✗ Error`) are red.
- **Background shading removed from response cards.** Only prompt cards retain the shaded background. Responses, errors, notes, and commands render without the colored rectangle.
- **Scroll via Ctrl+N (down) and Ctrl+P (up).** Emacs-style keybindings for scrolling the feed.
- **ESC cancels the current turn.** Pressing Escape while the model is thinking cancels the turn (same as Ctrl+C). Falls through to overlay close when not processing.
- **Context bar uses 1000-based token formatting.** Previously showed `128K` for 131,072 tokens (binary KiB). Now shows `131K` using decimal kilo (matching model documentation).
- **DeepSeek V4 Pro context window corrected to 1M.** Previously hardcoded at 128K in both provider presets and default model config.

### Fixed

- **Thinking buffer leaked across turns.** Old reasoning content from prior turns was appended into new thinking cards. Buffer now resets on `user_message` and `task_started` events.
- **Scroll viewport pinned to top of feed instead of bottom.** The scroll offset math was inverted (`Math.min` instead of `Math.max - offset`), causing the feed to show the oldest content and the `↑ N more` indicator on every render.
- **`?` key could not be typed into the prompt.** The global input listener consumed `?` as a no-op. Now passes through to the editor.
- **Splash was erased on settings changes.** Adding a note event for model/thinking changes hid the splash. Settings pickers now check `feed.isEmpty()` and update splash info inline when visible.
- **`isBashLoopError` returned `boolean | undefined`.** The optional chaining `error?.includes(...)` was not null-coalesced, caught by tsgo's stricter type checking.
- **Chat test hoisting with SWC.** `jest.mock()` calls hoisted by SWC referenced a `const` declared after them, causing a `ReferenceError` on initialization. Fixed by using `jest.requireMock()` to obtain the mock reference.

### Removed

- **Static model default for relay.** The hardcoded Qwen GGUF entry in `DEFAULT_MODELS.relay` is now a minimal fallback replaced at startup by the probed model list.
- **Per-line thinking duration counters.** Each line of thinking content no longer shows an individual elapsed-time suffix. Only the total elapsed time is shown once in the header.
- **Extra vertical padding around thinking cards.** Blank-line before/after padding was redundant with the inter-class spacing divider.
- **Redundant scroll keybindings.** PageUp/PageDown/Ctrl+Up/Ctrl+Down/End removed in favor of the simpler Ctrl+N/Ctrl+P. Local models that mix prose and tool calls in one response are no longer treated as fatal errors. Unsafe prose is stripped and tool calls execute normally. The stored assistant message is replaced (not duplicated) to avoid breaking strict providers.
- **Truncation guard (`finish_reason=length`).** When a model response is cut off by the output token limit, tool calls from that response are NOT executed (arguments may be truncated). A continuation nudge is injected so the model can recover. Three consecutive truncations abort the turn as `model_error`.
- **Better argument name aliasing for local models.** `edit`/`replace_in_file` now accept `file`, `filePath`, `old_str`, `oldText`, `old_text`, `search`, `original`, `new_str`, `newText`, `new_text`, `replacement`, `replace`. `write`/`create_file` accept `file`, `filePath`, `text`, `contents`, `body`. This lets local models that use Python-style names self-correct without failing.
- **Actionable argument errors.** When a known tool receives wrong argument names, the error message lists the expected names so the model can self-correct on the next step.
- **True unified diffs in patch previews.** `createUnifiedDiff` now produces proper LCS-based line-level diffs with context lines (`@@ -a,b +c,d @@` headers), common prefix/suffix trimming, and elision of long unchanged runs. Falls back to whole-region replace for very large inputs. Previously all lines were dumped with `-`/`+` prefixes.
- **Sub-agent orchestration is opt-in.** Sub-agents are disabled by default (`config.subagents.enabled`). When disabled, the entire planner pipeline is skipped and tasks run inline — keeping the common single-agent path fast and cheap. Explicit in-task delegation requests ("use parallel sub-agents…") override the default.
- **Overlapping file-scope safety for parallel orchestration.** Forced-parallel mode is downgraded to sequential when sub-task file scopes overlap, preventing concurrent mutations from corrupting each other. Read-only plans (e.g. repo recon) keep their parallelism.
- **Informational task detection.** Tasks like "explain X" or "why does Y fail?" are detected as informational, relaxing the `files_changed` verification contract so the model isn't pushed into making spurious edits just to satisfy the contract.
- **Read cache invalidation on mutation.** After any successful `edit`, `write`, or `bash` call, the read cache is cleared so subsequent reads return fresh content — preventing stale-read→edit mismatch loops.
- **Repo overhead budget cap.** Repo overhead in budget estimation is capped at 40% of the effective context window, preventing large repos from over-triggering orchestration for small tasks.
- **System message role fix for local chat templates.** Mid-conversation system messages (orientation, memory index, compaction notes) are converted to `user` role with a `[system context]` prefix, preventing ChatML variants from dropping them or resetting the conversation.
- **TUI: tool result truncation with expand/collapse.** Long tool results (60+ lines) are truncated with an expand indicator (`▸ N more lines (Enter to expand)`). Pressing `Enter` on an empty prompt expands the most recent truncated card; `Ctrl+E` toggles all expandable cards; `e` (empty prompt) toggles the latest card.
- **TUI: Ctrl+C "press again to quit" hint.** First Ctrl+C shows the hint in the status bar; it clears after a timeout if no second Ctrl+C arrives.
- **TUI: autocomplete solid background.** The slash-autocomplete dropdown now has a solid surface background so the transcript doesn't bleed through.
- **TUI: settings overlay uses app background.** Settings screen rows use the app background color (not surface) so the overlay blends with the rest of the TUI instead of painting the terminal a solid grey block.
- **TUI: renderer theme alignment.** The renderer's clear color is aligned with the resolved palette, preventing a hardcoded dark background from clashing with light terminal themes (e.g. Ghostty light mode).
- **TUI: animation timer tracking.** Animation timers are tracked and cancelled on shutdown / re-render to prevent timer leaks.
- **TUI: `Ctrl+E` expand all.** Toggles all expandable tool result cards at once.
- **TUI: thinking cards always show full text.** Thinking blocks no longer collapse/expand — they're always fully visible and naturally scrollable.
- **TUI: resume picker restores sessions in-process.** Selecting `/resume` now loads only the chosen session's JSONL transcript, rebuilds model-visible user/assistant context behind the stable system/skill prefix for prompt-cache friendliness, and leaves the picker list backed by lightweight session-index metadata.
- **TUI: resume picker metadata.** The picker now renders message count, status, and model from the session index, and session search includes provider names.
- **TUI: splash screen redesign.** The splash screen now features a styled HAMR wordmark with model-palette accent colors, a decorative box frame, tagline, and version line. Layout adapts to narrow/medium/wide terminals and cycles the wordmark accent color with each animation frame. Metadata is presented in a two-column grid with provider and uptime.
- **TUI: markdown rendering in thinking blocks.** Thinking cards now render structural markdown: ATX headings, bold headings, unordered/ordered lists, code fences, and inline formatting (bold, italic, code, links). Uses muted thinking-block styling to maintain a distinct visual identity from assistant messages.
- **`paste_context_range` tool.** New tool for context materialization: slices a range from the last user message using line numbers, anchor text, or byte offsets and writes it to a temp file (with sha256 verification). Supports multibyte unicode correctly and records the operation in the ledger.
- **Model context window auto-probe for relay/custom providers.** New `probeModelContextWindow()` queries `/v1/models` and `/v1/model` endpoints to discover the actual context window from server metadata (`max_context_length`, `max_model_len`, etc.). Runs at startup for non-cloud providers and overrides the configured context window. 3-second timeout, best-effort.
- **Gemma 3/4 native tool call support.** Gemma 3 and 4 models now use the `gemma_native` parser, which forces OpenAI-native `role: 'tool'` / `tool_call_id` conventions instead of XML-wrapped `<tool_response>` user messages. Gemma's chat template understands this format natively.
- **Runtime environment context injected into skill messages.** The model now receives repo path, home directory, username, and platform at the top of the skill message block. This grounds tool-call paths in the real environment instead of hallucinating `/home/user` or random absolute paths.
- **Bash enabled for all run modes.** Bash is now available in every mode (not just `patch`/`verify`). Read-only questions routinely need `git status`, `git diff`, and `git log` — without bash the model had no way to answer them and looped on directory listings.
- **Invalid-arguments errors are now recoverable.** When the model sends wrong argument names, the error is treated as recoverable so the model can self-correct on the next step instead of terminating the turn.
- **Image paste/drag support in TUI.** Pasting or dragging image files (png, jpg, gif, webp, bmp) into the prompt detects them, shows a compact `[📷 path]` indicator, and encodes them as multimodal content blocks at submit time. Vision models receive them as proper `image_url` content alongside the text prompt.
- **Session resume rebuilds transcript from persisted event log.** When resuming a session, the TUI now rebuilds visible transcript cards from the full persisted event log (`readSessionEvents` → `semanticEventsFromSessionEvents`) instead of showing a blank feed. The model's conversation context is restored behind the system prefix as before; the transcript shows the prior conversation above the prompt.
- **Multiple named themes.** Added 8 new TUI palettes alongside the default mono theme: gruvbox, kanagawa, catppuccin, nord, rose-pine, tokyo-night, pink, and dracula. Each has its own semantic color mapping, background, surface, border, and text colors.
- **`wordWrapLines` utility.** New text utility for word-boundary-aware line wrapping that matches OpenTUI's `wrapMode: 'word'` behavior. Falls back to character-level breaks for unbreakable words exceeding the width.
- **`applyFeedOperations` helper.** Extracted from inline TUI code: applies an `IncrementalFeedModel` render plan (append/update/remove operations) to a ScrollBox container with correct card index offset accounting.
- **Jest HOME isolation for tests.** A new setup file (`src/__tests__/helpers/jest-home.ts`) redirects `HOME` to a per-run temp directory so test suites don't flood the developer's real session index with fake sessions.

### Changed

- **Thinking tags (`<think>`/`<thinking>`) are stripped from stored content.** Echoing thinking tags back into conversation history wastes context and degrades multi-step behavior (Qwen's own guidance recommends stripping). Reasoning is preserved in `reasoningContent` separately, which also powers the think-only fallback (bug #114).
- **`temperature` defaults to 0.2.** Previously `temperature` was only sent when explicitly configured. Now defaults to 0.2 for all requests.
- **`stream_options.include_usage` enabled.** Streaming requests now request usage data from providers that support it.
- **Non-streaming tool call fallback.** When a stream degrades to a non-streaming response, native `tool_calls` from the response are captured.
- **Better tool call delta indexing.** Delta fragments are indexed by `delta.index`, matched by `delta.id` (for providers that repeat ids), or appended to the most recent entry — fixing fragmented arguments across separate tool call entries.
- **Bash execution environment increased.** `maxBuffer` raised to 2MB (was 256KB), `timeout` raised to 120s (was 30s) — letting longer builds and tests complete.
- **Verification timeout increased.** Non-full verification profiles now get 60s timeout (was 30s).
- **Recon sub-tasks are read-only.** Repo recon sub-tasks now use `verification: { level: 'none' }` instead of `files_changed`.
- **"use agents" regex excludes "use the agents.md".** The `AGENTS.md` file reference no longer triggers sub-agent delegation.
- **Repo recon intent excludes mutation tasks.** Tasks containing mutation verbs (fix, change, edit, write, etc.) are no longer hijacked into repo recon mode.
- **Preflight budget guard uses assembled request.** Token estimation before model calls now uses the fully assembled request (with orientation, memory index) instead of raw conversation messages, producing accurate budget checks.
- **TUI: sticky scroll behavior.** The ScrollBox's built-in `_hasManualScroll` handles pause/recovery without disabling `stickyScroll` entirely — scrolling back to the bottom auto-resumes following.
- **TUI: thinking state reset.** Thinking state clears on `task_started` and `user_message` events, preventing thinking blocks from appending across turns.
- **TUI: autocomplete draft cleared on non-slash input.** Prevents deadlocked input submission after backspacing past `/`.
- **TUI: settings text input accepts all printable characters.** Model names, URLs, and API keys with `:`, `_`, `-`, uppercase, etc. now work correctly in the settings panel.
- **CI/CD: deploy to Cloudflare Pages.** Docs deployment in GitHub Actions now uses `cloudflare/wrangler-action` instead of GitHub Pages.
- **Remember: system prompt instructs model to verify changes, plan before acting, and read files before editing.** Three new directives added to the system prompt.
- **TUI: expand/collapse removed.** Tool result cards and thinking cards no longer truncate or toggle. All content is always fully visible. Thinking cards collapse to a one-line preview on finalization (retitled from "Stopped thinking" to "Thought"). Removed `ExpandedState`, `Enter`-on-empty toggle, `Ctrl+E` toggle-all, `Ctrl+O` toggle, and `e` key toggle. The ScrollBox naturally handles long content — manual expand/collapse was unnecessary overhead.
- **TUI: renderer background set to transparent.** The renderer creation-time background is now `'transparent'` instead of a hardcoded dark color, so the terminal's native background shows through regardless of theme.
- **TUI: splash screen redesigned with token-stream frames.** The splash card now uses the model's token-stream character set (staggered rows) instead of the deleted `ai-core` visualizer. Layout is responsive and narrow-friendly with a colored accent bar, model name (middle-ellipsized for long GGUF filenames), and a single metadata row whose segments drop right-to-left when the terminal is narrow.
- **Removed `src/tui/ai-core.ts` and `src/tui/core-visual-profile.ts`.** These splash visualizer modules have been replaced by the token-stream-based splash card in `opentui-artifact-renderer.ts`.
- **TUI: settings and resume overlay lines update in place.** Navigation within overlays no longer triggers a full tree rebuild — only the text content of each line node is updated. Backdrop rows now include solid-fill `Text` children so the transcript behind the overlay doesn't bleed through below the modal frame. Prompt input is blurred while overlays are open.
- **TUI: prompt cursor changed to block (non-blinking).** The input cursor style changed from `line`/blinking to `block`/non-blinking for better visibility.
- **TUI: slash info panel pre-wrapped.** Slash-command info lines are pre-wrapped to the terminal width before layout, so the physical row count exactly matches what `footerLayoutHeight` computes — preventing overlap with the input.
- **TUI: ScrollBox vertical scrollbar hidden.** With `stickyScroll` locked to the bottom, the scrollbar was just visual noise that painted block-char columns over the right edge of result cards.
- **TUI: prompt cards render at full text brightness.** The user's own prompt words now use `pal.text` instead of `pal.textMuted`, making them the easiest thing to spot when scanning a long transcript.
- **TUI: root layout always uses full mode.** The compact-startup vs. full layout distinction is removed — the root structure stays the same between splash and transcript, so transitions are handled incrementally by the feed model without a full tree destroy+rebuild.
- **TUI: resize updates dimensions in place.** Terminal resize now updates root node dimensions and recalculates layout without destroying the tree, avoiding the blank-frame flicker of a full rebuild.
- **TUI: context budget bar restyled.** The bar now uses smooth cap-free half-blocks (`▰` filled / `▱` empty) instead of `▐`...`▌` delimiters.
- **TUI: cost suffix simplified.** Footer cost now shows only cumulative session spend; per-token in/out pricing is removed to reduce footer clutter.
- **TUI: resume picker frame matches settings modal.** The resume picker now uses the same box-drawing characters (`┌─┐` / `│` / `└─┘`) as the settings overlay. Selected rows use `→` instead of `>`. Footer hint simplified.
- **TUI: thinking cards finalized and collapsed in place.** On `tool_started`, `assistant_message`, `model_step_started`, or `task_finished`, the live thinking card is finalized in place (collapsed to first line, retitled "Thought") and a new thinking card starts for the next burst. This keeps thinking blocks in true stream order without moving them.
- **TUI: session filtering skips zero-event sessions.** Sessions with no recorded events are excluded from the resume picker to avoid dead-end restores.
- **TUI: home path normalization in prompt input.** Absolute paths under `$HOME` are normalized to `~/` prefix so the model's tools accept them. Also applies to pasted/dragged text.
- **TUI: streaming delta dedup for cumulative providers.** Some servers re-send the full accumulated text in each SSE delta. The presentation reducer now tracks `lastDeltaContent`/`lastDeltaReasoning` and strips the previously-seen prefix, preventing doubled paragraphs. The thinking card path handles this independently for reasoning streams.

### Fixed

- **Read cache nudges no longer mutate cached objects.** Shallow copies prevent "already read" guidance from sticking to future cache hits.
- **Orchestration: `shouldOrchestrate` compares to `'orchestrate'` (not `'orchestrated'`).** Fixed a mismatch that caused orchestration to never be triggered by budget estimation.
- **Orchestration: `totalKB` used directly instead of formatting `Math.ceil(...)` as a string.** The plan prompt now contains a numeric value.
- **Recovery: `skipTaskPush` prevents duplicate user messages.** Recovery re-entry no longer pushes the task again — the recovery manager already injected a nudge.
- **TUI: `footerLayoutHeight` accounts for slash info lines.** Slash-command info panels no longer overlap with the input.
- **TUI: theme detection falls through on null.** When the terminal doesn't answer the OSC theme query, falls through to `COLORFGBG` instead of treating null as a concrete answer.
- **TUI: `rootLayoutModeSignature` drops `prompt.length` from compact detection.** Having text in the prompt no longer forces a full UI tree rebuild — only event count, settings, and slash info matter.
- **TUI: slash completion acceptance.** Slash-command completions are tagged separately from file/model completions, so pressing Enter on `/resume` or another slash command dispatches it instead of merely inserting text into the prompt.
- **TUI: resume picker search input.** Typing while the resume picker is open filters sessions, and backspace edits the picker search query.
- **TUI: resume picker rendering and navigation.** The picker now renders plain fixed-width rows without embedded ANSI escape codes, uses ASCII frame markers, accepts common arrow/enter/tab/escape key variants, and prevents picker keys from leaking into the prompt textarea.
- **TUI: unsupported `/mouse` removed from slash autocomplete.** The menu no longer advertises a command without a real dispatcher.
- **Session ID collision defenses.** `EventStore` uses `startsWith('SQLITE_CONSTRAINT')` to catch `SQLITE_CONSTRAINT_UNIQUE` from better-sqlite3; `createSession` detects existing sessions before creation and throws a collision error; `SessionFactory` re-throws collision errors from `createStoreSession` instead of silently swallowing them; `generatePersistentSessionId` adds millisecond precision to match store-level granularity.
- **Robust bracketed paste handling.** Position-aware bracket detection prevents false matches inside pasted text; keypress events (keybindings, autocomplete, submit) are suppressed during raw bracketed paste to avoid double-insertion and interference; multi-byte UTF-8 and emoji are captured via `key.sequence` for printable characters.
- **Reasoning content forwarded through report pipeline.** `reasoningContent` is now forwarded through `RunTaskReport` → `ChatTurnReport` so downstream consumers (session store, TUI, run log) can access it independently of `finalAnswer`.
- **Prompt box overflow with word-wrap height simulation.** Replaced character-level wrap calculation with word-wrap simulation matching OpenTUI's `wrapMode: 'word'` behavior; long unbreakable words are force-broken; footer gets `overflow: hidden` to clip bleed into adjacent regions.
- **Verification contract resolved from mode.** `startTurn()` now resolves the verification contract from the mode when not explicitly set (`patch` → `files_changed`, `verify` → `verification_passed`, others → `none`) instead of silently skipping the check when the contract is `null`.
- **Centralized cost formatting with adaptive precision.** `formatCost` and `formatPricePer1M` moved to `src/tui/telemetry.ts` as shared exports; `formatCost` uses 2dp for ≥$100, 4dp for ≥$0.0001, and up to 10dp for sub-cent values; `formatPricePer1M` strips trailing zeros; eliminates duplicated local implementations and the `$0.00` display for small but real API calls.
- **Corrected model context window sizes.** DeepSeek V4 Pro/Flash preset changed from 1M to 128K (the 1M figure was a copy-paste error); added per-model window overrides with `resolveContextWindow()` as the single source of truth; canonical values added for `deepseek-chat` (128K) and `deepseek-reasoner` (64K).
- **TUI: overlay desync guards.** The render loop now detects when an overlay should exist but its nodes are missing (or vice versa) and forces a tree rebuild. Previously resizing with an overlay open could leave orphaned overlay nodes permanently blanking the view.
- **TUI: prompt input blurred under overlay.** When settings or the resume picker is open, the prompt input is blurred and its cursor hidden so the terminal cursor doesn't blink behind the modal. Refocused when the overlay closes.
- **TUI: root height synced on terminal resize.** The root node's height is now kept in sync with `renderer.height` on every render cycle, fixing Yoga layout drift when the terminal is resized while an overlay is open.
- **TUI: card index offset for session header.** The session header card occupies the first ScrollBox slot, so event index N maps to child index N+1. Without this offset, updated cards (notably the streaming thinking card) were re-inserted one slot too early and drifted above the previous prompt or tool call.
- **TUI: autocomplete draft cleared after bracketed paste.** After a bracketed paste completes, the autocomplete draft is cleared so a pasted path starting with `/` doesn't lock the prompt into slash-autocomplete mode.
- **TUI: preinserted prompt card suppression.** When the TUI pre-inserts a prompt card for immediate feedback, the event sink's subsequent `user_message` event is suppressed to avoid a duplicate prompt card in the transcript.
- **TUI: thinking card delta handles cumulative reasoning servers.** Some servers send the full accumulated reasoning text in each SSE delta. The thinking card path now computes a true delta by comparing the previously-sanitized body to the current one, and strips the prefix of a previously-finalized block when a new thinking burst starts.
- **TUI: `isModelHistoryBoundary` corrected.** The duplicate-model-history detection now treats any non-`model` item as a boundary (not just `user` items), preventing dedup collisions across tool results and other non-model history entries.

## [0.3.0-alpha.5]

### Fixed

- **Missing `extractTextContent` import in test files.**  \
  Three test suites (`context-hardening`, `deterministic-compaction`, `skills`) were missing the `extractTextContent` import from `../llm/types`, causing compilation failures. Added the import to all affected test files.

## [0.3.0-alpha.4]

### Added

- **Vision model support: `view_image` injects image content blocks.**  
  When `view_image` succeeds, the tool result is now exposed as a proper `image_url` content block in the conversation so vision-capable models (GPT-4V, Claude, etc.) can "see" the image. The image payload is stripped from token estimation to avoid 10–100× inflation vs real vision-tile costs.

- **LLM client: auto-detect `max_tokens` vs `max_completion_tokens`.**  
  The client now sends both `max_tokens` and `max_completion_tokens` by default and auto-detects which parameter is accepted on 400 errors, caching the correct choice per client instance. Fixes compatibility with newer OpenAI reasoning models (o1, o3, etc.) that reject `max_tokens`.

- **Token estimation: strip base64 image payloads.**  
  Context budget serialization now replaces large base64 image data with compact `[image:<bytes>]` placeholders before counting tokens, preventing catastrophic token inflation when images are present in conversation history.

### Changed

- **Tool definition path descriptions** updated to clarify that paths may be absolute or relative (not repo-relative only).
- **`AgentMessage.content`** type widened from `string` to `ChatContent` to support multimodal content arrays.

## [0.3.0-alpha.3]

### Fixed

- **Config: fix invalid/undefined `thinking` values in `resolveActive` and `configFromParsed`.**  
  Malformed or missing thinking settings in hamr config files would cause crashes. Added safe fallbacks that default to off when the value is not a recognized string.

- **Prompt box: fix rendering issues and thinking block formatting.**  
  Multi-line prompt input now correctly handles the layout recalculation path without triggering full UI tree rebuilds, and thinking blocks render without visual glitches.

- **Fix `thinking` default in config.**  
  The `--thinking` CLI flag default is now properly wired through the config layer instead of being dropped.

## [0.3.0-alpha.2]

### Fixed

- **Reasoning sanitization: fix missing spaces when stripping `<think>` / `<thinking>` tags.**  
  Well-formed protocol XML tags (`<think>`, `<thinking>`, `<tool_call>`, `<invoke>`, `<function>`, `<parameter>`) were removed with an empty replacement string, which silently joined adjacent words when the tag was flush against surrounding text. Tags are now replaced with a space, preventing word-joining; duplicate whitespace is collapsed in a final cleanup pass. Affected three files: `stripToolCallMarkup` in the TUI display path, `sanitizeReasoning` in the tool-call repair path, and `assistantVisibleContent` in session formatting.

- **TUI prompt box: fix disappearing/overflow glitch on multi-line input.**  
  The prompt input height was included in `rootLayoutModeSignature`, causing a full UI tree rebuild every time the prompt wrapped to a new visual line. Removed `inputHeight` from the signature so height changes are handled in-place via the existing yoga layout recalculation path. Added `overflow: hidden` to the input frame box to prevent text overflow during the brief window before layout recalculation completes.

## [0.3.0-alpha.1]

Initial alpha release.
