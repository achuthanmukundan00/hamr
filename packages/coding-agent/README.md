# hamr

Minimal terminal coding agent. Read, edit, bash, and write with any LLM.

## Install

```bash
npm install -g @skaft/hamr
```

Then run it inside a project:

```bash
hamr
```

Set an API key before the first run, or use `/login` inside hamr for subscription providers:

```bash
export ANTHROPIC_API_KEY=sk-ant-...
hamr
```

## Providers

hamr supports the following LLM providers out of the box:

| Provider | Config |
|---|---|
| Anthropic (Claude) | `ANTHROPIC_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| Google Gemini | `GEMINI_API_KEY` |
| AWS Bedrock | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` + `AWS_REGION` |
| Mistral | `MISTRAL_API_KEY` |
| Local (Ollama, LM Studio) | Set endpoint in `.hamr.toml` |

Switch provider and model on the fly with `--model` or the in-session model picker.

## Usage

```
hamr [task]                 Open interactive TUI (optionally with an initial task)
hamr run --task "..."       Non-interactive one-shot task
hamr --model claude-sonnet-4-6
hamr --provider openai --model gpt-4o
hamr --session <id>         Resume a saved session
hamr --continue             Continue the most recent session
hamr --version
hamr --help
```

Inside the TUI:

- Type a task and press Enter to submit
- `/help` — list available slash commands
- `/model` — switch provider or model
- `/login` — authenticate with a subscription provider
- `/plan` — enter plan mode for multi-step tasks
- Ctrl+C / Escape — cancel the current turn

## Configuration

hamr reads `.hamr.toml` from the project directory and `~/.config/hamr/settings.toml` for global defaults. Common settings:

```toml
[provider]
name = "anthropic"
model = "claude-sonnet-4-6"

[tui]
theme = "hamr"           # hamr | dark | light | gruvbox | catppuccin | nord | …
alternateScreen = false  # true for alternate screen (no native scrollback)
```

## Extensions

Extend hamr with TypeScript modules that add tools, slash commands, event hooks, or custom UI cards. Place extension files under `.hamr/extensions/` or reference them in `.hamr.toml`:

```toml
[extensions]
paths = ["./my-tools.ts"]
```

See [docs/extensions.md](docs/extensions.md) for the full extension API.

## Sessions

hamr persists every session as a JSONL file under `~/.hamr/sessions/`. Sessions can be resumed, branched, and exported to HTML. Use `hamr --session <id>` to resume by ID, or `hamr --continue` for the most recent.

## Acknowledgments

hamr builds on [pi](https://github.com/badlogic/lemmy) by Mario Zechner and the `sexy-tui-rs` renderer. See [NOTICE.md](NOTICE.md) for full attribution.

## License

MIT
