# Providers

Hamr supports OpenAI-compatible and Anthropic Messages providers.
You can configure multiple providers and switch between them at runtime.

## Provider Protocols

| Protocol             | Description                                                        |
| -------------------- | ------------------------------------------------------------------ |
| `openai-compatible`  | Any service with an OpenAI-compatible `/chat/completions` endpoint |
| `anthropic-compatible` | Real Anthropic Messages API adapter via `/v1/messages`           |

## Built-in Provider Presets

Hamr ships with presets for these providers:

| Provider ID  | Protocol              | Base URL                         | Auth                 |
| ------------ | --------------------- | -------------------------------- | -------------------- |
| `relay`      | openai-compatible     | `http://127.0.0.1:1234/v1`       | none                 |
| `custom`     | openai-compatible     | user-configured                  | optional             |
| `deepseek`   | openai-compatible     | `https://api.deepseek.com/v1`    | `DEEPSEEK_API_KEY`   |
| `openrouter` | openai-compatible     | `https://openrouter.ai/api/v1`   | `OPENROUTER_API_KEY` |
| `groq`       | openai-compatible     | `https://api.groq.com/openai/v1` | `GROQ_API_KEY`       |
| `anthropic`  | anthropic-compatible  | `https://api.anthropic.com`      | `ANTHROPIC_API_KEY`  |
| `mistral`    | openai-compatible     | `https://api.mistral.ai/v1`      | `MISTRAL_API_KEY`    |
| `together`   | openai-compatible     | `https://api.together.xyz/v1`    | `TOGETHER_API_KEY`   |

## Configuring a Provider

All examples use the current multi-provider config format. The legacy `[provider]` format from v0.1–v0.3 still works but is not recommended for new configs.

### Local Relay

```toml
[active]
provider = "relay"
model = "Qwen3.6-35B-A3B-UD-IQ3_XXS.gguf"

[providers.relay]
enabled = true
base_url = "http://127.0.0.1:1234/v1"
```

The `base_url` line is only needed when not running on the default port. Set `model` to the exact ID your server reports from `GET /models`.

### DeepSeek

```toml
[active]
provider = "deepseek"
model = "deepseek-chat"

[providers.deepseek]
enabled = true
api_key_env = "DEEPSEEK_API_KEY"
```

### OpenRouter

```toml
[active]
provider = "openrouter"
model = "deepseek/deepseek-chat"

[providers.openrouter]
enabled = true
api_key_env = "OPENROUTER_API_KEY"

[providers.openrouter.headers]
HTTP-Referer = "https://github.com/skaft-software/hamr"
X-Title = "Hamr"
```

### Groq

```toml
[active]
provider = "groq"
model = "llama-3.3-70b-versatile"

[providers.groq]
enabled = true
api_key_env = "GROQ_API_KEY"
```

### Anthropic

```toml
[active]
provider = "anthropic"
model = "claude-sonnet-4-5-20250929"

[providers.anthropic]
enabled = true
compatibility = "anthropic-compatible"
api_key_env = "ANTHROPIC_API_KEY"
```

Anthropic uses the real Messages API (`POST /v1/messages`) with `x-api-key` auth. System prompts map to the top-level `system` field. Tool use uses the native Anthropic tool format.

### Custom OpenAI-compatible

```toml
[active]
provider = "myserver"
model = "local-model"

[providers.myserver]
enabled = true
base_url = "http://127.0.0.1:8080/v1"

[[providers.myserver.models]]
id = "local-model"
display_name = "My Local Model"
context_window = 131072
supports_thinking = false
```

## API Key Configuration

Prefer `api_key_env` over `api_key`. The environment variable is never written to disk.

```toml
[providers.deepseek]
api_key_env = "DEEPSEEK_API_KEY"  # reads from process.env.DEEPSEEK_API_KEY
```

## Custom Headers

Header values can reference environment variables using `$VAR` or `${VAR}` syntax:

```toml
[providers.relay.headers]
"CF-Access-Client-Id" = "${CF_ACCESS_CLIENT_ID}"
"CF-Access-Client-Secret" = "${CF_ACCESS_CLIENT_SECRET}"
```

Hamr resolves variables from `process.env` at runtime. Unset vars omit the header entirely.

## Token Pricing & Session Spend

Cloud providers have preset token pricing. Override in config:

```toml
[providers.deepseek]
input_price_per_1m_tokens = 0.27   # USD per 1M input tokens
output_price_per_1m_tokens = 1.10  # USD per 1M output tokens
```

Default pricing per provider:

| Provider   | Input ($/1M) | Output ($/1M) |
| ---------- | ------------ | ------------- |
| DeepSeek   | $0.27        | $1.10         |
| Groq       | $0.59        | $0.79         |
| Anthropic  | $3.00        | $15.00        |
| OpenRouter | varies       | varies        |

When pricing is configured, the TUI shows a session spend indicator at the bottom: `Spend: $0.004`. Local providers show `Spend: local`.

## Switching Providers

From the TUI: press `/`, type `settings`, navigate to the **Model** tab. Changes persist to the active config file.

## Live Provider Smoke Tests

Smoke tests verify end-to-end connectivity. Run from source only — they require API keys and are not run in CI.

```bash
HAMR_LIVE_PROVIDER=relay bun run smoke:provider
HAMR_LIVE_PROVIDER=deepseek bun run smoke:provider
HAMR_LIVE_PROVIDER=openrouter bun run smoke:provider
HAMR_LIVE_PROVIDER=anthropic bun run smoke:provider
HAMR_LIVE_PROVIDER=custom HAMR_CUSTOM_BASE_URL=http://127.0.0.1:1234/v1 bun run smoke:provider
```

## Known Limitations

- Mistral and Together are available as presets but lack dedicated smoke tests.
- Streaming is implemented in the shared OpenAI-compatible client but the Anthropic adapter only supports non-streaming requests.
