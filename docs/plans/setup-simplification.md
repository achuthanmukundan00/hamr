# Relay Feature Inventory & Docs Rewrite Plan

> Running document — add/edit freely as the discussion evolves.
> Target audience: **complete terminal beginner** who just wants local AI to work with their coding agent.

---

## 1. Current Features (as-implemented in code)

### Core Gateway
- [x] `/v1/chat/completions` — OpenAI chat proxy with canonicalization
- [x] `/v1/responses` — OpenAI Responses API (full SSE lifecycle, tool calls, reasoning, `previous_response_id` conversation continuation)
- [x] `/v1/messages` — Anthropic Messages proxy
- [x] `/v1/messages/count_tokens` — Anthropic token counting
- [x] `/v1/embeddings` — embedding normalization
- [x] `/v1/rerank` (and `/rerank`) — rerank normalization
- [x] `/v1/completions` — legacy completions shim
- [x] `/v1/models` — OpenAI-compatible model list with context sizes
- [x] Streaming SSE normalization (repair, re-chunk, event lifecycle)
- [x] Field policy engine (pass-through / strip-with-warning / reject)
- [x] Request/response canonicalization pipeline
- [x] Tool call normalization across providers

### Auto Model Lifecycle (Gateway Mode)
- [x] Lazy loading — models start on first request
- [x] Auto shutdown after idle timeout
- [x] Session-aware context clearing (new `session-id` → restart model)
- [x] Orphan cleanup (kills stale llama-server from previous instances)
- [x] Dynamic port allocation per model
- [x] Eager switching (kill old → start new)
- [x] Circuit breaker (stop retrying models that fail repeatedly)
- [x] Max warm models limit

### Cloud Mode
- [x] Proxy to external APIs (Gemini, DeepSeek, OpenAI, etc.)
- [x] Per-model base URL + auth header (from env vars)
- [x] `/v1/models` reflects cloud model capabilities

### Gateway → Cloud Fallback
- [x] Unknown models forwarded to a cloud relay container
- [x] Cloud-forwarded responses saved to local store for `previous_response_id`

### Observability
- [x] `GET /` — HTML status dashboard (dark theme, model table, queue cards, lifecycle state)
- [x] `GET /health` — JSON health check
- [x] `GET /relay/status` — full lifecycle + queue JSON
- [x] `GET /relay/metrics` — request counts, latencies, error rates
- [x] `GET /relay/jobs` — job queue state
- [x] `GET /relay/lifecycle` — per-model lifecycle
- [x] `GET /relay/stats` — request history
- [x] `GET /relay/capabilities` — capability registry
- [x] Structured logging (JSON, configurable levels)
- [x] Truncation diagnostics (request/response size tracking)

### Auth & Rate Limiting
- [x] API key auth (Bearer token or `x-api-key` header)
- [x] Per-key rate limiting (each token gets independent bucket)
- [x] Constant-time key comparison
- [x] Bind-to-all-interfaces safety check (refuses to start without API_KEY)

### Setup Wizard
- [x] Interactive TUI (Python, `scripts/setup-tui.py`)
- [x] Three modes: local (GPU models), cloud (proxy to external APIs), BYO (existing server/Ollama)
- [x] GPU/VRAM detection (`scripts/probe-gpu.sh`)
- [x] Curated model catalog (`docs/model-catalog.json`) — 25+ community models with quant info
- [x] Real-time HuggingFace size lookup (no stale catalog sizes)
- [x] VRAM fit filtering with 1.5GB headroom
- [x] GGUF file size estimation (`scripts/size-model.py`)
- [x] Auto-provisioning: downloads llama.cpp binary from GitHub releases, picks Vulkan/CUDA/ROCm asset for detected GPU
- [x] Start script generation with correct flags (GPU layers, KV cache type, MoE offloading, --jinja, draft models)
- [x] Model download with resume + GGUF magic validation
- [x] `.env` generation (API_KEY, RELAY_MODEL_MAP)
- [x] Docker compose generation (Linux, host networking + GPU mounts)
- [x] Cloud mode: multi-provider picker (OpenAI, Anthropic, DeepSeek, Groq) with secure API key entry
- [x] BYO mode: Ollama auto-detection, existing-server passthrough
- [x] Non-TTY fallback (numbered menus for CI/scripts)

### Containerization
- [x] Dockerfile (multi-stage, Node.js)
- [x] docker-compose.yml (host networking, PID namespace, GPU mounts)
- [x] Cloudflare Tunnel sidecar

### Testing
- 262 tests total:
  - 9 streaming SSE lifecycle tests (new)
  - 42 PTY/TUI tests (Python)
  - 13 TUI fallback tests
  - 8 catalog tests
  - 7 sizing tests
  - 6 provisioning tests
  - 4 size-parse tests
  - ~173 other node tests (auth, errors, lifecycle, streaming, tools, field policy, Anthropic, embeddings, etc.)
- 7 pre-existing failures (canonical-parity, model-switch/prefix-cache, responses, tool-torture)

---

## 2. The Plan: Two Changes + Polish

### Change 1: One-liner install script (`scripts/install.sh`)

New file. User runs:
```bash
curl -fsSL https://raw.githubusercontent.com/achuthanmukundan00/relay/main/scripts/install.sh | bash
```

The script:
1. Checks for Node.js — if missing, prints a friendly "Download it from nodejs.org" and exits
2. Clones relay to `~/relay` (or `$RELAY_DIR` if set)
3. Runs `npm install`
4. Launches the wizard: `node --experimental-strip-types src/main.ts setup`

That's it. No TUI changes. No flow changes. Just getting INTO the wizard with one command.

### Change 2: Kill `--experimental-strip-types` from `npm start`

In `package.json`, change:
```json
"start": "tsx src/main.ts"
```
`tsx` is already a devDependency. No new deps. User-facing command becomes `npm start` with no flags.

---

## 3. TUI Polish (same steps, warmer feel)

All the same screens, all the same logic. Just visual + copy improvements:

### 3a. Step counter
Add `(Step 1/7)` etc to each screen title. Removes the "how long is this going to take?" anxiety.

Before: `📦  Picking models`
After:  `📦  Picking models  (Step 3/7)`

Steps for local full setup: Welcome → Mode pick → Catalog → llama.cpp → Models folder → Tuning → Done (7 steps)

### 3b. Collapse the banner after welcome
Currently the ASCII logo + hardware specs bar takes 11 lines on every single screen. After the welcome screen, shrink it to a 1-line mini-banner:

```
  relay · MacBook Pro · 24 GB GPU · 64 GB RAM
```

More room for actual content. Less visual noise.

### 3c. Catalog screen: warmer help text
Currently:
> You've got ~24 GB to play with. Anything marked ✓ fits will run smoothly. I've pre-ticked a great default — press Enter to take it, or Space to choose your own.

This is already good. Just add: "You can always re-run setup to add more models later."

### 3d. Tuning step: show what's happening
Currently the tuning is silent — models get sized and scripts written with no feedback. Add per-model checkmarks:

```
  ✓ Qwen3.6-35B-A3B  ·  131K context  ·  Vulkan  ·  MoE offloaded
  ✓ Gemma-3-27B      ·  131K context  ·  Vulkan  ·  vision ready
```

### 3e. Done screen: show the curl command
Add the exact copy-paste test command:

```
  Test it:
    curl http://127.0.0.1:1234/v1/models

  Or point your agent at  http://127.0.0.1:1234/v1
```

### 3f. Welcome screen: shorter
Cut the current 6 lines of intro copy to 3. The logo already says "relay". People don't read paragraphs.

---

## 4. Docs Rewrite Structure (proposed)

Same as current but reorganized for discoverability:

```
docs/
├── index.md                    # "What is Relay?" — one paragraph + architecture diagram
├── quickstart.md               # 3 steps: download, setup, connect agent
├── installation.md             # Detailed per-platform (macOS, Linux, Docker)
├── configuration.md            # Every env var, what it does, defaults
├── models.md                   # How model management works, adding custom models
├── agents.md                   # Connecting Cursor, opencode, Claude Code, etc.
├── cloud-mode.md               # Using Relay as a cloud proxy
├── observability.md            # Status page, metrics, logs
├── api-compatibility.md        # Endpoint reference
├── troubleshooting.md          # Common errors and fixes
├── deploy-public.md            # HTTPS, tunnels, sharing
├── deploy-systemd.md           # Background service on Linux
├── architecture.md             # Internal design for contributors
└── faq.md                      # "Why not just use llama.cpp directly?" etc.
```

---

## 5. Summary

Two files changed, same wizard, much gentler entry point:

| What | Effort | Impact |
|---|---|---|
| `scripts/install.sh` (one-liner) | 30 min | User goes from zero to wizard in one paste |
| `package.json` `"start": "tsx"` | 1 min | No `--experimental-strip-types` anywhere user-facing |
| Step counter in titles | 20 min | "I know how much is left" |
| Mini-banner after welcome | 30 min | 10 more lines of content per screen |
| Tuning checkmarks | 15 min | User sees what just happened |
| Done screen curl command | 5 min | User can verify it works |
| Shorter welcome copy | 10 min | Less to read, same message |

Everything else — cloud mode, BYO, catalog, model download, llama.cpp auto-provision, Docker compose, `previous_response_id` continuation — stays exactly as it is.
