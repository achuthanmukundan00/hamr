# Getting Started

## Install

```sh
npm install -g @skaft/hamr
```

> **Tip:** Hamr works best with Relay — the local inference gateway that handles model lifecycle, graceful switching, and prefix-cache pre-warming. See the [Relay docs](/guide/relay) for setup.

Hamr requires Bun at runtime for the interactive TUI. If Bun isn't installed:

```sh
curl -fsSL https://bun.sh/install | bash
```

## Quick Start

Hamr works out of the box with Relay (or any OpenAI-compatible local server) running at `http://127.0.0.1:1234/v1`. Start your local inference server, then:

```sh
hamr doctor
```

## Create Project Config

Run `hamr config init` to scaffold a `.hamr.toml`, or copy the example:

```sh
cp .hamr.toml.example .hamr.toml
```

Edit `.hamr.toml` to pick your provider and model. Example for local Relay:

```toml
[active]
provider = "relay"
model = "Qwen3.6-35B-A3B-UD-IQ3_XXS.gguf"
thinking = "off"

[providers.relay]
enabled = true
name = "Relay"
compatibility = "openai-compatible"
base_url = "http://127.0.0.1:1234/v1"

[[providers.relay.models]]
id = "Qwen3.6-35B-A3B-UD-IQ3_XXS.gguf"
display_name = "Qwen3.6 35B"
context_window = 131072
supports_thinking = false
```

### Cloud Providers

For hosted APIs, set `api_key_env` to the environment variable holding your key:

```toml
[active]
provider = "deepseek"
model = "deepseek-v4-pro"
thinking = "off"

[providers.deepseek]
enabled = true
name = "DeepSeek"
compatibility = "openai-compatible"
base_url = "https://api.deepseek.com/v1"
api_key_env = "DEEPSEEK_API_KEY"
```

Built-in providers: `relay` (local), `deepseek`, `openai`, `anthropic`, `openrouter`.

### Custom OpenAI-Compatible Endpoint

```toml
[providers.custom]
enabled = true
name = "Custom"
compatibility = "openai-compatible"
base_url = "http://127.0.0.1:8080/v1"

[[providers.custom.models]]
id = "my-local-model"
display_name = "My Local Model"
context_window = 131072
supports_thinking = false
```

Use `api_key_env = "MY_KEY"` for auth endpoints. Set `compatibility = "anthropic-compatible"` for Anthropic-format APIs.

### Custom Headers

Add arbitrary headers per provider (Cloudflare Access, proxies, etc.):

```toml
[providers.relay.headers]
"CF-Access-Client-Id" = "your-id.access"
"CF-Access-Client-Secret" = "your-secret"
```

### Config Layers

Hamr merges config left-to-right (later wins):

1. Built-in defaults (Relay on `localhost:1234`)
2. `~/.config/hamr/config.toml` (global)
3. `.hamr.toml` (project-local)

## Check The Setup

```sh
hamr doctor            # quick check — config, skills, sessions
hamr doctor --full     # full check — probes /models and sends a test chat completion
```

## First Session

```sh
hamr inspect           # project overview
hamr inspect --skills  # list discovered skills
hamr inspect --docs    # browse project documentation
hamr ask --question "Summarize this repository in five bullets."
hamr run --task "Fix the failing test" --yes
```

Interactive TUI:

```sh
hamr chat
```

Inside chat:

```txt
hamr> /settings      — switch providers, models, skills, mcp
hamr> /tools         — list available tools
hamr> /budget        — token usage stats
hamr> /test-provider — probe current provider
hamr> /verify        — run verification command
hamr> /undo-last-edit
hamr> /exit
```

## Develop From Source

```sh
git clone git@github.com:skaft/hamr.git
cd hamr
bun install
bun run build
bun run hamr -- --help
```

## Tool-Call Parsing

For local models emitting non-standard tool calls, set `tool_call_parser`:

```toml
[providers.relay]
tool_call_parser = "qwen3_xml"
```

Hamr auto-detects the parser from your model name. See [Tool-Call Parsing](/guide/tool-call-parsing) for the full parser matrix.

## Skills

Place `SKILL.md` files in `~/.hamr/skills/<name>/` (global) or `.hamr/skills/<name>/` (project). Hamr auto-discovers and injects them into the agent context.

Disable auto-discovered skills:

```sh
hamr run --task "fix the test" --no-skills
```

See [Skills](/guide/skills) for details.

## Next Steps

- [Configuration](/guide/configuration) — full config reference
- [Providers](/guide/providers) — provider-specific setup
- [Commands](/guide/commands) — CLI reference
- [Relay Setup](/guide/relay) — local inference setup
