# Hamr Rust Rewrite — Architecture Specification

> **FOR SUBAGENTS: READ THIS FIRST.**
> Every Rust file in this workspace is a direct 1:1 mirror of a TypeScript file in `../../packages/`.
> Your job: read the TS source, port it to Rust using the conventions below, produce compilable code.

## Quick Start

1. Read this spec (crate map, conventions, dependency order).
2. Each Rust file's doc comment tells you exactly which TS file to port.
3. Port that TS file to Rust following the **Porting Conventions**.
4. Build in order: `hamr-ai` → `hamr-harness` → `hamr-agent`.
5. Run `cargo check -p <crate>` after each completed file.

---

## Crate Map (BUILD ORDER)

| Step | TS Package | Rust Crate | Purpose |
|---|---|---|---|
| **1** | `packages/ai` | `hamr-ai` | LLM provider abstraction, streaming, model registry, core types |
| **2** | `packages/agent` | `hamr-harness` | Agent loop, compaction, session storage, extension infrastructure |
| **3** | `packages/coding-agent` | `hamr-agent` | CLI, built-in tools, hamr extensions, TUI modes, session manager |
| ext | `packages/tui` | sexy-tui-rs | Already ported. Integrate via `tui` feature flag later. |
| 4 | everything else | `hamr-support` + root | Docs, release scripts, config |

**CRITICAL**: Build in order. `hamr-ai` compiles alone. `hamr-harness` depends on `hamr-ai`. `hamr-agent` depends on both.

---

## Porting Conventions

### TypeScript → Rust Type Mappings

| TypeScript | Rust |
|---|---|
| `string` | `String` |
| `number` | `f64` (general) or `u64` (tokens/sizes) |
| `boolean` | `bool` |
| `TypeBox.Static<typeof Schema>` | `#[derive(Serialize, Deserialize, JsonSchema)]` struct |
| `TypeBox.TSchema` | `schemars::JsonSchema` bound |
| `Record<string, T>` | `HashMap<String, T>` |
| `T[]` / `Array<T>` | `Vec<T>` |
| `T \| undefined` / `T?` | `Option<T>` |
| `T \| null` | `Option<T>` (Rust has no null) |
| discriminated union (`{ type: "foo" } \| { type: "bar" }`) | `enum` with `#[serde(tag = "type")]` |
| `Promise<T>` | `async fn` or `impl Future<Output = T>` |
| `async () => T` | `Pin<Box<dyn Future<Output = T> + Send>>` |
| callback: `(x: T) => void` | `Arc<dyn Fn(T) + Send + Sync>` |
| callback: `(x: T) => Promise<R>` | `Arc<dyn Fn(T) -> Pin<Box<dyn Future<Output = R> + Send>> + Send + Sync>` |
| `AbortSignal` | `tokio::sync::watch::Receiver<bool>` |
| `EventEmitter` / event listener | `tokio::sync::mpsc::UnboundedSender<Event>` |
| `Map<K, V>` | `HashMap<K, V>` |
| `Set<T>` | `HashSet<T>` |
| `Date` / timestamp `number` | `chrono::DateTime<Utc>` |
| `RegExp` | `regex::Regex` |
| `Buffer` | `Vec<u8>` |
| `process.env` | `std::env::var()` |

### Naming

| TypeScript | Rust |
|---|---|
| `camelCase` identifiers | `snake_case` identifiers |
| `camelCase` filenames (`agent-loop.ts`) | `snake_case` filenames (`agent_loop.rs`) |
| PascalCase types/interfaces | PascalCase structs/enums |
| `I` prefix (interface) | Drop the `I` prefix |
| `type Foo = ...` | `pub type Foo = ...` or `pub struct Foo { ... }` |

### Serde

Use `#[serde(rename_all = "camelCase")]` on all serialized types that cross JSON boundaries. TypeScript uses camelCase in JSON.

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FooBar {
    pub some_field: String,  // serializes as "someField"
}
```

### Discriminated Unions

The TS codebase uses tagged unions heavily. In Rust:

```typescript
// TypeScript
type Event =
  | { type: "start", model: string }
  | { type: "done", message: AssistantMessage }
```

```rust
// Rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    #[serde(rename = "start")]
    Start { model: String },

    #[serde(rename = "done")]
    Done { message: AssistantMessage },
}
```

### Async

- All LLM calls and file I/O use `async fn` with `tokio`.
- Callback-based async uses `Pin<Box<dyn Future>>`.
- Long-running operations check a `watch::Receiver<bool>` for abort signals.
- Use `tokio::select!` for racing operations against abort signals.

### Error Handling

- Use `thiserror` for error types. Every crate defines its own error enum.
- Extension event handlers return `Result<T>`. The runner catches errors and emits `ExtensionError`.
- **Never** `unwrap()` or `expect()` in production paths. Use `?` or explicit `match`.

### Dependencies to Use

| TS dependency | Rust crate |
|---|---|
| `typebox` | `schemars` (derive `JsonSchema`) |
| `cross-spawn` | `tokio::process::Command` |
| `diff` | `similar` |
| `better-sqlite3` | `rusqlite` (with `bundled` feature) |
| `chalk` | ANSI escape sequences via `crossterm::style` |
| `marked` / markdown | `pulldown-cmark` |
| `highlight.js` | `syntect` or `tree-sitter-highlight` |
| `semver` | `semver` crate |
| `yaml` | `serde_yaml` |
| `glob` | `glob` crate |
| `ignore` | `ignore` crate |
| `undici` | `reqwest` |
| `jiti` (dynamic TS import) | `rhai` (for agent-authored extensions) |
| `playwright` | `headless_chrome` or `chromiumoxide` |

### Where to Find TS Source

All TS source is at `../../packages/` relative to the `rust/` directory.
Each Rust file's doc comment points to the exact TS file it mirrors.

---

## File → File Map (Critical Modules)

These are the highest-priority files to port first — they define types used everywhere:

### Phase 1: hamr-ai types (no deps, safe to parallelize)
| Rust file | TS source |
|---|---|
| `hamr-ai/src/types.rs` | `packages/ai/src/types.ts` |
| `hamr-ai/src/stream.rs` | `packages/ai/src/stream.ts` |
| `hamr-ai/src/utils/event_stream.rs` | `packages/ai/src/utils/event-stream.ts` |

### Phase 2: hamr-harness (depends on hamr-ai)
| Rust file | TS source |
|---|---|
| `hamr-harness/src/types.rs` | `packages/agent/src/types.ts` |
| `hamr-harness/src/agent_loop.rs` | `packages/agent/src/agent-loop.ts` |
| `hamr-harness/src/agent.rs` | `packages/agent/src/agent.ts` |

### Phase 3: hamr-agent extension types (depends on hamr-harness)
| Rust file | TS source |
|---|---|
| `hamr-agent/src/core/extensions/types.rs` | `packages/coding-agent/src/core/extensions/types.ts` |
| `hamr-agent/src/core/extensions/runner.rs` | `packages/coding-agent/src/core/extensions/runner.ts` |

### Phase 4: Everything else (parallelize freely within each crate)

---

## Testing

- Each crate has a `tests/` directory.
- Port tests from the corresponding `packages/*/test/` directory.
- Run with `cargo test -p <crate>`.

---

## What's Already Done

- [x] Cargo workspace with 4 crates
- [x] Full module tree mirroring TS source
- [x] `hamr-ai::types` — core LLM types: Message, Model, Tool, Context, AssistantMessageEvent
- [x] `hamr-ai::models` — model registry with provider detection and cost data
- [x] `hamr-ai::providers` — 9 providers implemented: anthropic, openai-completions, openai-responses, openai-codex-responses, google, google-vertex, amazon-bedrock, mistral, azure-openai-responses, cloudflare
- [x] `hamr-ai::providers::register_builtins` — all 9 providers registered + auto-init via `hamr_ai::init()`
- [x] `hamr-ai::stream` — top-level stream/complete entry points
- [x] `hamr-ai::images` — image generation registry
- [x] `hamr-ai::oauth` — OAuth registry with reset support
- [x] `hamr-ai::providers::transform_messages` — message normalization
- [x] `hamr-ai::providers::openai_responses_shared` — shared Responses API logic
- [x] `Model.compat` field (`Option<serde_json::Value>`) — wired into openai-responses, openai-completions, google providers
- [x] `hamr-harness::types` — core agent types
- [x] `hamr-harness::session` — session storage (JSONL, memory, SQLite)
- [x] `hamr-harness::compaction` — compaction logic
- [x] `hamr-harness::truncate` — output truncation with byte/line limits
- [x] `hamr-agent::core::tools` — bash, read, edit, write, grep, find, ls
- [x] `hamr-agent::core::extensions` — types, runner, loader (with Rhai support)
- [x] `hamr-agent::core::session_manager` — session management
- [x] `hamr-agent::core::compaction` — context compaction with `should_compact`, `find_cut_point`, `prepare_compaction`
- [x] `hamr-agent::core::model_registry` — model registry with OAuth wiring
- [x] `hamr-agent::core::settings_manager` — settings management
- [x] `hamr-agent::core::trust_manager` — trust/path security
- [x] `hamr-agent::hamr::extensions::subagents` — delegate_subagents tool registered with full schema
- [x] `hamr-agent::hamr::extensions::memory` — fact store + FTS5
- [x] `hamr-agent::hamr::extensions::providers` — provider extension
- [x] `hamr-agent::hamr::repair` — tool call repair with has_tool_calls, get_assistant_text, get_thinking_text
- [x] `hamr-agent::modes::print_mode` — with ImageContent support
- [x] `hamr-agent::modes::rpc` — RPC mode types and client
- [x] `hamr-agent::cli` — args, startup, session picker
- [x] `sexy-tui-rs` — full TUI framework (already ported)
- [x] Feature flags for providers, extensions, TUI, Rhai
- [x] 3034 tests passing, 0 failures
- [x] Production hardening: Model.compat, provider registration, error handling

## Remaining Work (lower priority)

- [ ] Google Vertex ADC: service-account JWT minting from GOOGLE_APPLICATION_CREDENTIALS
- [ ] OpenAI Codex Responses: WebSocket transport fallback (SSE-only path works)
- [ ] Provider retry logic with exponential backoff (5xx, 429 rate limits)
- [ ] Persistent editor: full TUI integration (requires TUI types)
- [ ] Interactive mode: full TUI component wiring
- [ ] RPC mode: event/response channel wiring
- [ ] Dynamic provider registration from config (StreamSimpleArgs → ApiStreamFunction bridge)
- [ ] Port remaining integration tests from TS (301 test files in TS vs 185 test modules in Rust)
- [ ] ~59 remaining TODOs in provider compat, TUI wiring, and extension infrastructure
