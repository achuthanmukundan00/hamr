---
title: Hamr
description: Local-first coding agent. Built for Relay, llama.cpp, and OpenAI-compatible inference.
---

# Hamr

**Local-first coding agent.** Built for local models on consumer GPUs.

## Quick Start

```sh
npm install -g @skaft/hamr
hamr
```

Start your inference server (Relay, llama.cpp, LM Studio, Ollama, vLLM) at its default port, then run `hamr`. From the TUI: `/login` → "Use a custom/self-hosted endpoint" to configure.

## Why Hamr

- **Relay-first** — recommended inference gateway: model lifecycle, prefix-cache pre-warming
- **Local-first** — designed for your GPU, not a cloud bill
- **Pi SDK** — embed the agent in your own tools
- **OpenAI-compatible** — works with any `/chat/completions` endpoint

## Docs

- [Getting Started](/guide/getting-started)
- [Configuration](/guide/configuration)
- [Providers & Endpoints](/guide/providers)
- [Relay Setup](/guide/relay)
- [Commands & TUI](/guide/commands)
- [Architecture](/guide/architecture)
