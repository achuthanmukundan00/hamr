# Finish hamr restructure (synax → hamr) — Design

Date: 2026-06-17 · Status: approved (verbal "go for it")

## Goal
`hamr` boots with synax's full experience + the 7 features, on **pi-as-engine**. Not stock pi.

## Non-goals
- **No sexy-tui-rs swap.** Keep pi-tui (it's better). *Theme* it.
- **No new themes.** Reconstruct synax `default`/`kawaii`/`pinkOut` as-is.
- **No Rust rewrite** (separate, spec 007).
- **Preserve pi's native terminal scrolling** (trackpad + mouse). Must not break.

## Architecture
pi-as-engine. Boot = pi `main()` (`packages/coding-agent/src/cli.ts`) + `hamrBuiltinExtension`
(`packages/coding-agent/src/hamr/extension.ts`). All hamr features = pi extension hooks/tools +
theme + provider registrations. **Source-of-truth for behavior/visuals = `~/workspace/git/synax/src`.**

## Decisions (user-refined)
- **Relay** = OpenAI-compatible provider. Complexity masked, native. Models auto-detect via endpoint (like synax). No vendored relay server.
- **Orchestration** = lean parallel+sequential dispatcher, simple code (pi-style). Feeds an observable dashboard. Feel = Claude Code, more observable.
- **Theming** = reconstruct synax themes as-is + model-aware default coloring.

## Current state (~half)
- Solid: FTS5 memory store path, local-model tool-call parsers (wired via extension).
- Gaps: memory invisible to user; theming partial; indicator = glyph frames (want rainbow words);
  only sequential `delegate_subagent`; no real `HandoffManager`; relay = config only;
  dead `packages/hamr/` orphan + `reference/` graveyard.

## Workstreams (acceptance per item)
- **W0 Baseline + cleanup.** Commit WIP. Delete `packages/hamr/` + `reference/` (single source of truth).
  Refactor `extension.ts` monolith → per-feature modules (`memory.ts`, `indicator.ts`, `subagents.ts`,
  `handoff.ts`, theme reg); `extension.ts` only composes. AC: `npm run verify` green; `extension.ts` thin.
- **W1 Memory visibility** (task #1). `save_memory`/`search_memory`/`handoff_memory` tool calls render in
  TUI with stored AND retrieved content. Port HolographicMemory richness from synax. AC: live run shows
  the tool call + content; FTS5 search returns; index injected.
- **W2 Theming.** Reconstruct synax `default`/`kawaii`/`pinkOut` on pi's theme; model-aware default
  (status bar + accents recolor by active model). AC: visual parity w/ synax; model switch recolors;
  **native scroll works (trackpad + mouse)** (task #2).
- **W3 Working indicator.** Rainbow shimmering **words** (not glyphs). AC: animated word shimmer while model works.
- **W4 Subagents.** Lean parallel + sequential dispatcher; feeds ctrl+o dashboard live
  (status / elapsed / result). AC: parallel fan-out runs; dashboard observable; sequential still works.
- **W5 Handoff.** Structured `HandoffManager` over FTS5 (beyond `handoff_memory`). AC: structured handoff
  manifest tool; consumable by subagents.
- **W6 Relay.** OpenAI-compatible provider with endpoint model auto-detect, complexity masked.
  AC: point at relay endpoint → models appear → switch works.

## Fanout
- Parallel, disjoint files: **W2 theming**, **W6 relay**.
- Serial, owned centrally (touch `extension.ts` + new modules): **W0 → W1, W3, W4, W5**.
- Per-workstream gate: `npm run typecheck && npm run lint && npm run build && npm test` + live boot smoke.

## Verification (done =)
- `npm run verify` green.
- `node packages/coding-agent/dist/cli.js` boots the hamr look (not pi): model-aware color, rainbow word
  indicator, native scroll.
- Memory tool calls visible with content.
- Subagents: parallel run visible in ctrl+o dashboard.
- Relay endpoint auto-detects models.
- No `packages/hamr/`, no `reference/`.
