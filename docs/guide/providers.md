# Providers

Hamr supports OpenAI-compatible and Anthropic Messages providers.
You can configure cloud providers, self-hosted endpoints, and switch between them at runtime.

## Provider Protocols

| Protocol              | Description                                                        |
| --------------------- | ------------------------------------------------------------------ |
| `openai-completions`  | Any service with an OpenAI-compatible `/chat/completions` endpoint |
| `anthropic-messages`  | Anthropic Messages API via `/v1/messages`                          |

## Cloud Providers

Hamr ships with built-in models for these cloud providers. Set the API key via environment variable or `/login`:

| Provider    | Env Var              |
| ----------- | -------------------- |
| Anthropic   | `ANTHROPIC_API_KEY`  |
| OpenAI      | `OPENAI_API_KEY`     |
| DeepSeek    | `DEEPSEEK_API_KEY`   |
| OpenRouter  | `OPENROUTER_API_KEY` |
| Groq        | `GROQ_API_KEY`       |
| Mistral     | `MISTRAL_API_KEY`    |
| Together    | `TOGETHER_API_KEY`   |

Full list at `/login` → "Use an API key" in the TUI.

## Self-Hosted / Local Endpoints

For local inference servers (llama.cpp, LM Studio, Ollama, vLLM, Relay) or custom OpenAI/Anthropic-compatible proxies, use **two** configuration paths:

### 1. TUI (recommended)

From the Hamr TUI: `/login` → "Use a custom/self-hosted endpoint"

This opens a form where you can:
- Pick a preset (LM Studio, llama.cpp, Ollama, vLLM, or custom)
- Set the base URL and API type
- Configure optional API key and custom headers
- Auto-discover available models from the endpoint

Configuration is saved to `~/.hamr/agent/models.json`.

### 2. models.json (manual / SDK)

Create or edit `~/.hamr/agent/models.json`:

```json
{
  "providers": {
    "lm-studio": {
      "baseUrl": "http://localhost:1234/v1",
      "api": "openai-completions",
      "apiKey": "not-needed",
      "models": [
        { "id": "qwen2.5-coder-7b" }
      ]
    }
  }
}
```

For servers that don't understand the `developer` role or `reasoning_effort`:

```json
{
  "providers": {
    "llama.cpp": {
      "baseUrl": "http://localhost:8080/v1",
      "api": "openai-completions",
      "apiKey": "not-needed",
      "compat": {
        "supportsDeveloperRole": false,
        "supportsReasoningEffort": false
      },
      "models": [
        { "id": "local-model" }
      ]
    }
  }
}
```

The file reloads each time you open `/model`. Edit during session; no restart needed.

### Headers & API Keys

API key and header values support environment variable interpolation (`$VAR`, `${VAR}`) and shell commands (`!command`):

```json
{
  "providers": {
    "custom-proxy": {
      "baseUrl": "https://proxy.example.com/v1",
      "api": "openai-completions",
      "apiKey": "$PROXY_API_KEY",
      "headers": {
        "X-Custom-Auth": "!op read 'op://vault/item/secret'"
      },
      "models": [
        { "id": "proxy-model" }
      ]
    }
  }
}
```

## Switching Providers

From the TUI: press `/`, type `model`, or use `/login` to configure credentials.
Changes to `models.json` are picked up on next `/model` open without restart.

## Token Pricing

Cloud providers have preset token pricing. Local/self-hosted providers default to zero cost. Override in `models.json`:

```json
{
  "providers": {
    "my-provider": {
      "baseUrl": "...",
      "api": "openai-completions",
      "apiKey": "...",
      "models": [
        {
          "id": "model-id",
          "cost": { "input": 0.27, "output": 1.10, "cacheRead": 0, "cacheWrite": 0 }
        }
      ]
    }
  }
}
```

Cost is per million tokens.
