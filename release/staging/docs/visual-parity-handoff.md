# Visual Parity Handoff — hamr vs synax

**Date:** 2026-06-17
**Branch:** `feat/finish-hamr-restructure`
**Goal:** Make hamr's default TUI indistinguishable from synax `@skaft/hamr`

---

## ✅ Completed (this session)

### Bug B — Splash refreshes on model change
- **File:** `src/modes/interactive/interactive-mode.ts`
- Added `splashRendered: boolean` and `splashRenderedModelKey: string` tracking fields (lines 272-273)
- `renderSplashScreen()` (line ~1688) sets these after rendering
- Added `refreshSplashIfLive()` method (~line 1770): compares current model key to cached key; if different, clears `chatContainer` and re-renders splash
- Hooked into `updateEditorBorderColor()` (line ~3656) so every model switch or thinking-level change refreshes the splash

### Bug C — PROMPT cards with per-model glyph + brand color
- **File:** `src/modes/interactive/components/user-message.ts`
- Rewrote `UserMessageComponent`: now accepts `modelAccent?` (hex) and `modelGlyph?` (string)
- Renders `{glyph} PROMPT` heading in model brand color when `modelAccent && theme.modelAdaptive`, theme accent otherwise
- Always shows the heading when `modelGlyph` is available — user can see which model was prompted even after model switches
- `interactive-mode.ts`: `addMessageToChat()` captures `modelAccent` + `modelGlyph` from session model at call time, passes to both UserMessageComponent and AssistantMessageComponent

### Bug D — Emoji glyph auto-detection
- **File:** `src/modes/interactive/theme/theme.ts`
- Added `detectGlyphTier()` (line ~270): auto-detects terminal capability
  - Emoji: iTerm2, Apple Terminal, Kitty, Ghostty, WezTerm, Warp, Tabby, Hyper, VS Code, Cursor, Windsurf, xterm-kitty
  - Nerd: Alacritty, Rio, truecolor/24bit COLORTERM
  - Unicode: fallback for everything else
  - Ascii: dumb TERM or NO_COLOR=1
- `HAMR_EMOJI_MODEL_GLYPHS` / `HAMR_NERD_FONT` env vars act as overrides (opt-up)
- `modelGlyph()` now uses `getGlyphTier()` instead of hardcoded env-var precedence

### Bug G — Dashboard model-adaptive color
- **File:** `src/hamr/dashboard.ts`
- Header "⚒ Hamr Dashboard" and "Commands" section title now use model brand color when `theme.modelAdaptive` is on
- Falls back to theme accent when modelAdaptive is off

### Bug E — (prior session) Removed duplicate Working indicator
- Removed in-feed loader creation from two sites in interactive-mode.ts
- Footer's independent `animationTimer` for rainbow "Working..." was verified safe

### Bug A — (prior session) Extension shortcut conflicts
- Removed cosmetic `[Extension issues]` warning for `restrictOverride: false` built-in overrides in `runner.ts`
- These are intentional context-scoped key reuse (e.g. ctrl+d = session.delete AND tree.filter)

---

## 🔧 In Progress

### Bug F — Scroll-while-typing transcript
- **Status:** Investigated, NOT implemented
- **Complexity:** 200-400 line architectural change in `@hamr/tui`

**Root cause:** The current TUI rendering engine (`packages/tui/src/tui.ts`) has no scrollable container concept. The viewport is always bottom-anchored (`doRender()`, line ~1214). All children (chat + editor + footer) are rendered as one flat buffer by `Container.render()`. There's no way to "pin" the editor at the bottom while scrolling the chat history independently.

**What's needed:**
1. Add a `TUI.scrollOffset` field that shifts which content rows are visible in the viewport
2. Introduce a concept of "fixed" vs "scrollable" children in Container (or a new `ScrollableContainer`)
3. Render the scrollable region from `viewportTop - scrollOffset` while fixed children render at the bottom of the terminal
4. Keyboard handlers: PgUp/PgDown/Ctrl+Up/Ctrl+Down adjust scrollOffset; auto-reset to 0 when new content arrives or user scrolls to bottom
5. Mouse wheel support (if terminal supports SGR mouse)

**Files to touch:**
- `packages/tui/src/tui.ts` — viewport calculation in `doRender()`, new scroll state, input dispatch
- `packages/tui/src/container.ts` — OR a new `ScrollableView` component
- `packages/coding-agent/src/modes/interactive/interactive-mode.ts` — keybinding setup for scroll

**Synax reference:** `src/tui/components/event-feed.ts` has its own `_scrollOffset` + `_userScrolled` + `scrollUp()`/`scrollDown()`/`scrollToBottom()` — but synax's event-feed is a single component that manages its own scroll within a fixed-height viewport. Synax's TUI layout keeps the editor pinned separately.

---

## 📋 Remaining

### Low-hanging fruit
- **Help text rebrand:** `package-manager-cli.ts` now says "Update hamr" / "hamr update"
- **README claims "9 color themes":** Only `default`, `kawaii`, `pinkOut` ship in `src/modes/interactive/theme/`. Either ship more themes or fix the claim.
- **GitHub URLs in migrations.ts:** Still point to `skaft-software/hamr` repo

### Medium effort
- **Version string:** Currently `0.79.6` (inherited from the Pi fork base). Consider bumping to a Hamr-specific version (e.g. `1.0.0`).
- **CHANGELOG.md:** Historical entries predate the fork and reference original Pi package names. These are archival and can stay, but new entries should use `@hamr/*` naming.
- **npm-shrinkwrap.json:** References `hamr-tui` in resolved URLs — auto-generated, updates on next `npm install`
- **P0 Checklists:** Write comprehensive Demo Gates + Brand Gate docs (user provided detailed checklist)

### Large effort (deferred)
- **Architecture simplification:** Toward `hamr theme + sexy-tui-rs + agent` — needs design doc
- **Full `@skaft/hamr` rebrand:** Package naming, exports, SDK docs
- **Scroll-while-typing:** See "In Progress" above

---

## Architecture Context

```
hamr monorepo
├── packages/tui/         → @hamr/tui     (fork of the pi TUI, shared terminal UI components)
├── packages/ai/          → @hamr/ai      (model providers, API clients)
├── packages/agent/       → @hamr/agent   (agent loop, tools, sessions)
└── packages/coding-agent → @skaft/hamr  (CLI, TUI mode, extensions)

synax (reference)
└── src/tui/              → HamrTheme, EventFeed, StatusBar, AgentDashboard
```

- **Theme default:** `APP_NAME === "hamr"` → `getDefaultTheme()` returns `"hamr"` → loads `hamr.json`
- **Startup flow:** `InteraciveMode.run()` → `rebindCurrentSession()` → `renderInitialMessages()` → splash OR restored messages
- **Model change flow:** `cycleModel()` / `cycleThinkingLevel()` → `updateEditorBorderColor()` → now also calls `refreshSplashIfLive()`
- **Message rendering:** Live events + session restore both go through `addMessageToChat()` which now captures model accent/glyph at call time
