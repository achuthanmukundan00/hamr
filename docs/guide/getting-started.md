# Getting Started

## Install

```sh
npm install -g @skaft/hamr
```

> **Tip:** Hamr works best with [Relay](/guide/relay), the recommended local inference gateway. Also works with LM Studio, llama.cpp, Ollama, vLLM, and any OpenAI-compatible endpoint.

## Quick Start

Hamr defaults to Relay (or any OpenAI-compatible local server) at `http://127.0.0.1:1234/v1`. Start your inference server, then:

```sh
hamr doctor            # quick check — config, skills, sessions
hamr doctor --full     # probe /models and send a test completion
```

No config file is required for local Relay at the default address.

## Config Files

Hamr uses these config paths:

1. `~/.config/hamr/config.toml` — global, machine-wide settings
2. `.hamr.toml` — project-local, walks up from cwd until found
3. `~/.hamr/agent/models.json` — custom providers and models

Use the TUI (`/login`) to configure providers, or create files by hand.

### Local inference (Relay)

Put this in `~/.config/hamr/config.toml` or `.hamr.toml`:

```toml
[active]
provider = "relay"
model = "your-model-name.gguf"
thinking = "off"

[providers.relay]
enabled = true
base_url = "http://127.0.0.1:1234/v1"
```

Set `model` to the exact ID your server reports from `GET /models`. The `base_url` line is only needed if you're not running on the default port.

### Cloud providers

Set `api_key_env` to the environment variable holding your key:

```toml
[active]
provider = "deepseek"
model = "deepseek-chat"

[providers.deepseek]
enabled = true
api_key_env = "DEEPSEEK_API_KEY"
```

Built-in provider IDs: `relay`, `deepseek`, `openai`, `anthropic`, `openrouter`, `groq`, `mistral`, `together`.

### Custom OpenAI-compatible endpoint

```toml
[providers.custom]
enabled = true
base_url = "http://127.0.0.1:8080/v1"

[[providers.custom.models]]
id = "my-local-model"
display_name = "My Local Model"
context_window = 131072
supports_thinking = false
```

### Custom headers (Cloudflare Access, proxies)

```toml
[providers.relay.headers]
"CF-Access-Client-Id" = "${CF_ACCESS_CLIENT_ID}"
"CF-Access-Client-Secret" = "${CF_ACCESS_CLIENT_SECRET}"
```

## First Session

```sh
hamr run --task "Fix the failing test" --yes
```

Interactive TUI:

```sh
hamr
```

Inside the TUI:

```txt
/settings      — switch providers, models, skills, mcp
/tools         — list available tools
/budget        — token usage stats
/test-provider — probe current provider
/verify        — run verification command
/undo-last-edit
/exit
```

## Develop From Source

```sh
git clone https://github.com/skaft-software/hamr.git
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
