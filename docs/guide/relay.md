# Relay Setup

Relay is the recommended inference gateway for Hamr. It manages model lifecycle, graceful switching, and prefix-cache pre-warming for local LLMs. Hamr communicates with Relay through a standard OpenAI-compatible API.

## Requirements

Relay exposes an OpenAI-compatible API at a base URL. The default is:

```
http://127.0.0.1:1234/v1
```

Required endpoints:
- `GET /models` — model discovery
- `POST /chat/completions` — inference

## Configuring Hamr for Relay

### Method 1: TUI (recommended)

From the Hamr TUI: `/login` → "Use a custom/self-hosted endpoint"

Select the appropriate preset (LM Studio, llama.cpp, Ollama, vLLM, or Custom) and enter your Relay base URL. Hamr auto-discovers available models and saves the configuration to `~/.hamr/agent/models.json`.

### Method 2: models.json (manual)

Create or edit `~/.hamr/agent/models.json`:

```json
{
  "providers": {
    "relay": {
      "baseUrl": "http://127.0.0.1:1234/v1",
      "api": "openai-completions",
      "apiKey": "not-needed"
    }
  }
}
```

If Relay requires an API key or custom headers (e.g., Cloudflare Access):

```json
{
  "providers": {
    "relay": {
      "baseUrl": "http://127.0.0.1:1234/v1",
      "api": "openai-completions",
      "apiKey": "$RELAY_API_KEY",
      "headers": {
        "CF-Access-Client-Id": "$CF_ACCESS_CLIENT_ID",
        "CF-Access-Client-Secret": "$CF_ACCESS_CLIENT_SECRET"
      }
    }
  }
}
```

For fine-grained model control, list models explicitly:

```json
{
  "providers": {
    "relay": {
      "baseUrl": "http://127.0.0.1:1234/v1",
      "api": "openai-completions",
      "apiKey": "not-needed",
      "compat": {
        "supportsDeveloperRole": false,
        "supportsReasoningEffort": false
      },
      "models": [
        {
          "id": "Qwen3.6-35B-A3B-UD-IQ3_XXS.gguf",
          "name": "Qwen3.6 35B",
          "contextWindow": 131072,
          "reasoning": false,
          "input": ["text"]
        }
      ]
    }
  }
}
```

## Tool-Call Parsing

For local models emitting non-standard tool calls, Hamr auto-detects the parser from the model name. To override, set `tool_call_parser` in your TOML config or use a model override in `models.json`. See [Tool-Call Parsing](/guide/tool-call-parsing) for the full parser matrix.

## Verification

Once Relay is running:

```sh
hamr doctor --full
```

This probes `/models` and sends a test completion to confirm end-to-end connectivity.

## Switching Models

Use `/model` in the TUI to browse and switch between discovered models. Hamr queries the Relay endpoint at startup and whenever `/model` is opened.
