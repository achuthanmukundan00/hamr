#!/usr/bin/env python3
"""
Generate per-provider Rust model files + `models_generated.rs` aggregator.

Mirrors the architecture of `packages/ai/src/models.generated.ts`:
parses the canonical monolithic TS source (hamr-main) and produces one
Rust source file per provider under `providers/model_data/`, plus a small
`models_generated.rs` that calls each one.

Usage:
    python3 rust/scripts/generate_models.py

Output:
    rust/hamr-ai/src/providers/model_data/<provider>_models.rs  (35 files)
    rust/hamr-ai/src/providers/model_data/mod.rs
    rust/hamr-ai/src/models_generated.rs

Re-run whenever model data changes upstream.
"""

import re
import sys
from pathlib import Path

# Paths relative to this script
SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent  # hamr repo root
HAMR_MAIN_ROOT = REPO_ROOT.parent / "hamr-main"  # canonical TS source

# Canonical monolithic source (preferred)
TS_MONOLITH_CANONICAL = HAMR_MAIN_ROOT / "packages" / "ai" / "src" / "models.generated.ts"

# Fallback: per-provider files in the hamr repo
TS_DIR_FALLBACK = REPO_ROOT / "packages" / "ai" / "src" / "providers"
TS_AGGREGATOR_FALLBACK = REPO_ROOT / "packages" / "ai" / "src" / "models.generated.ts"

# Rust output
RS_OUT_DIR = REPO_ROOT / "rust" / "hamr-ai" / "src" / "providers" / "model_data"
RS_AGGREGATOR = REPO_ROOT / "rust" / "hamr-ai" / "src" / "models_generated.rs"


# ---------------------------------------------------------------------------
# TS → Rust mapping tables
# ---------------------------------------------------------------------------

API_MAP: dict[str, str] = {
    "openai-completions": "Api::OpenAiCompletions",
    "mistral-conversations": "Api::MistralConversations",
    "openai-responses": "Api::OpenAiResponses",
    "azure-openai-responses": "Api::AzureOpenAiResponses",
    "openai-codex-responses": "Api::OpenAiCodexResponses",
    "anthropic-messages": "Api::AnthropicMessages",
    "bedrock-converse-stream": "Api::BedrockConverseStream",
    "google-generative-ai": "Api::GoogleGenerativeAi",
    "google-vertex": "Api::GoogleVertex",
}

THINKING_KEY_MAP: dict[str, str] = {
    "off": "ModelThinkingLevel::Off",
    "minimal": "ModelThinkingLevel::Minimal",
    "low": "ModelThinkingLevel::Low",
    "medium": "ModelThinkingLevel::Medium",
    "high": "ModelThinkingLevel::High",
    "xhigh": "ModelThinkingLevel::XHigh",
}


# ---------------------------------------------------------------------------
# TS parsing helpers
# ---------------------------------------------------------------------------

# Provider key in monolithic file: one tab, optionally quoted identifier.
#   \t"amazon-bedrock": {
#   \tanthropic: {
PROVIDER_KEY_RE = re.compile(r'^\t("?)([a-zA-Z0-9_./@:-]+)\1\s*:\s*\{\s*$')

# Model key in monolithic file: two tabs, always quoted.
#   \t\t"amazon.nova-2-lite-v1:0": {
MODEL_KEY_MONOLITH_RE = re.compile(r'^\t\t"([^"]+)":\s*\{\s*$')

# End-of-model marker in monolithic file.
#   \t\t} satisfies Model<...>,
MODEL_SATISFIES_MONOLITH_RE = re.compile(r'^\t\t\}\s*satisfies\s+Model<')

# Provider close in monolithic file.
#   \t},
PROVIDER_CLOSE_RE = re.compile(r'^\t\},?\s*$')

# Model key in per-provider files: one tab, always quoted.
#   \t"claude-3-5-haiku-20241022": {
MODEL_KEY_PERFILE_RE = re.compile(r'^\t"([^"]+)":\s*\{\s*$')

# End-of-model marker in per-provider files.
#   \t} satisfies Model<...>,
MODEL_SATISFIES_PERFILE_RE = re.compile(r'^\t\}\s*satisfies\s+Model<')


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def parse_model_fields(body_lines: list[str]) -> dict[str, str]:
    """
    Parse lines of a single model entry body (between `{` and `} satisfies Model<...>`).
    Returns field_name → raw_value string.
    """
    text = "\n".join(body_lines).strip()
    fields: dict[str, str] = {}
    idx = 0

    while idx < len(text):
        # Skip whitespace and commas
        while idx < len(text) and text[idx] in " \t\n\r,":
            idx += 1
        if idx >= len(text):
            break

        # Read key
        key_start = idx
        if text[idx] == '"':
            idx += 1
            while idx < len(text) and text[idx] != '"':
                idx += 1
            idx += 1
        else:
            while idx < len(text) and re.match(r"[a-zA-Z_$]", text[idx]):
                idx += 1
        key = text[key_start:idx].strip().strip('"')
        if not key:
            idx += 1
            continue

        # Skip ':'
        while idx < len(text) and text[idx] in " \t\n\r":
            idx += 1
        if idx < len(text) and text[idx] == ":":
            idx += 1
        while idx < len(text) and text[idx] in " \t\n\r":
            idx += 1
        if idx >= len(text):
            break

        # Read value (handles nested braces/brackets/quotes)
        ch = text[idx]
        val_start = idx
        if ch == "{":
            depth = 1
            idx += 1
            while depth > 0 and idx < len(text):
                if text[idx] == "{":
                    depth += 1
                elif text[idx] == "}":
                    depth -= 1
                idx += 1
        elif ch == "[":
            depth = 1
            idx += 1
            while depth > 0 and idx < len(text):
                if text[idx] == "[":
                    depth += 1
                elif text[idx] == "]":
                    depth -= 1
                idx += 1
        elif ch in "\"'":
            quote = ch
            idx += 1
            while idx < len(text):
                if text[idx] == "\\":
                    idx += 2
                    continue
                if text[idx] == quote:
                    idx += 1
                    break
                idx += 1
        else:
            while idx < len(text) and text[idx] not in ",\n\r}":
                idx += 1
        fields[key] = text[val_start:idx].strip()

    return fields


def parse_monolithic_ts(path: Path) -> dict[str, list[dict[str, str]]]:
    """
    Parse the canonical monolithic `models.generated.ts` file.
    Returns {provider_name: [model_field_dict, ...]}.
    """
    raw = path.read_text(encoding="utf-8").splitlines()

    # Find start: "export const MODELS = {"
    start_idx = None
    for i, line in enumerate(raw):
        if line.strip().startswith("export const MODELS"):
            start_idx = i
            break
    if start_idx is None:
        print(f"ERROR: could not find 'export const MODELS' in {path}", file=sys.stderr)
        return {}

    providers: dict[str, list[dict[str, str]]] = {}
    current_provider: str | None = None
    in_provider = False
    in_model = False
    model_id: str | None = None
    body_lines: list[str] = []

    i = start_idx
    while i < len(raw):
        line = raw[i]

        if in_model:
            if MODEL_SATISFIES_MONOLITH_RE.match(line):
                fields = parse_model_fields(body_lines)
                fields.setdefault("id", model_id or "unknown")
                if current_provider is not None:
                    providers.setdefault(current_provider, []).append(fields)
                in_model = False
                model_id = None
                body_lines = []
                i += 1
                continue
            else:
                body_lines.append(line)
                i += 1
                continue

        if in_provider:
            if PROVIDER_CLOSE_RE.match(line) and not line.strip().startswith("//"):
                in_provider = False
                current_provider = None
                i += 1
                continue

            m_model = MODEL_KEY_MONOLITH_RE.match(line)
            if m_model:
                in_model = True
                model_id = m_model.group(1)
                body_lines = []
                i += 1
                continue

            i += 1
            continue

        m_prov = PROVIDER_KEY_RE.match(line)
        if m_prov:
            current_provider = m_prov.group(2)
            in_provider = True
            providers.setdefault(current_provider, [])
            i += 1
            continue

        i += 1

    return providers


def parse_perfile_ts(path: Path) -> list[dict[str, str]]:
    """Parse a single per-provider `*.models.ts` file."""
    raw = path.read_text(encoding="utf-8").splitlines()
    models: list[dict[str, str]] = []
    i = 0
    while i < len(raw):
        line = raw[i]
        m = MODEL_KEY_PERFILE_RE.match(line)
        if m:
            model_id = m.group(1)
            i += 1
            body_lines: list[str] = []
            while i < len(raw):
                if MODEL_SATISFIES_PERFILE_RE.match(raw[i]):
                    fields = parse_model_fields(body_lines)
                    fields.setdefault("id", model_id)
                    models.append(fields)
                    i += 1
                    break
                body_lines.append(raw[i])
                i += 1
            continue
        i += 1
    return models


def parse_aggregator(path: Path) -> dict[str, str]:
    """
    Parse the aggregator `models.generated.ts`.
    Returns {provider_name: export_const_name}.
    """
    text = path.read_text(encoding="utf-8")
    providers: dict[str, str] = {}
    models_match = re.search(r"export const MODELS\s*=\s*\{(.*?)\}\s*as const", text, re.DOTALL)
    if models_match:
        body = models_match.group(1)
        for m in re.finditer(r'"([a-zA-Z0-9_./@:-]+)"\s*:\s*([A-Z_]+)', body):
            providers[m.group(1)] = m.group(2)
    return providers


# ---------------------------------------------------------------------------
# TS value → Rust code generators
# ---------------------------------------------------------------------------

def parse_input_array(v: str) -> str:
    v = v.strip()
    if not v.startswith("[") or not v.endswith("]"):
        return "vec![]"
    inner = v[1:-1].strip()
    if not inner:
        return "vec![]"
    items = []
    for part in re.split(r",\s*", inner):
        p = part.strip().strip("'\"").lower()
        if p == "text":
            items.append("InputModality::Text")
        elif p == "image":
            items.append("InputModality::Image")
    return "vec![" + ", ".join(items) + "]"


def parse_cost(v: str) -> str:
    fields: dict[str, str] = {}
    for m_val in re.finditer(r"(\w+)\s*:\s*([^,\n}]+)", v):
        fields[m_val.group(1)] = m_val.group(2).strip()

    def fmt_val(val: str) -> str:
        val = val.strip()
        if "." in val:
            return val
        return val + ".0"

    return (
        f"ModelCost {{ "
        f"input: {fmt_val(fields.get('input', '0'))}, "
        f"output: {fmt_val(fields.get('output', '0'))}, "
        f"cache_read: {fmt_val(fields.get('cacheRead', '0'))}, "
        f"cache_write: {fmt_val(fields.get('cacheWrite', '0'))} "
        f"}}"
    )


def parse_headers(v: str) -> str:
    entries = []
    for m_val in re.finditer(r'"([^"]*)"\s*:\s*"([^"]*)"', v):
        entries.append(f'("{esc(m_val.group(1))}".into(), "{esc(m_val.group(2))}".into())')
    if not entries:
        return "None"
    return "Some(HashMap::from([" + ", ".join(entries) + "]))"


def parse_thinking_map(v: str) -> str:
    v = v.strip()
    if not v.startswith("{") or not v.endswith("}"):
        return "None"
    inner = v[1:-1].strip()
    if not inner:
        return "None"
    entries = []
    for part in re.split(r",\s*", inner):
        m_val = re.match(r"\s*(\w+)\s*:\s*(.+)\s*$", part)
        if not m_val:
            continue
        level_key = m_val.group(1).lower()
        val_str = m_val.group(2).strip()
        rust_level = THINKING_KEY_MAP.get(level_key)
        if rust_level is None:
            continue
        if val_str == "null":
            entries.append(f"({rust_level}, None)")
        else:
            inner_val = val_str.strip("'\"")
            entries.append(f'({rust_level}, Some("{esc(inner_val)}".into()))')
    if not entries:
        return "None"
    return "Some(HashMap::from([" + ", ".join(entries) + "]))"


def model_to_rust(fields: dict[str, str], model_id: str) -> str:
    """Convert parsed model fields into a Rust Model struct expression."""
    name = fields.get("name", model_id).strip('"')
    api_str = fields.get("api", "").strip('"')
    provider = fields.get("provider", "").strip('"')
    base_url = fields.get("baseUrl", "").strip('"')
    reasoning = fields.get("reasoning", "false")
    input_arr = fields.get("input", "[]")
    cost_str = fields.get("cost", "{}")
    context_window = fields.get("contextWindow", "0")
    max_tokens = fields.get("maxTokens", "0")
    headers_str = fields.get("headers")
    thinking_str = fields.get("thinkingLevelMap")

    api_rust = API_MAP.get(api_str)
    if api_rust is None:
        print(f"  WARNING: unknown API '{api_str}' for model '{model_id}'", file=sys.stderr)
        api_rust = "Api::AnthropicMessages"

    parts = [
        f'id: "{esc(model_id)}".into()',
        f'name: "{esc(name)}".into()',
        f"api: {api_rust}",
        f'provider: "{esc(provider)}".into()',
        f'base_url: "{esc(base_url)}".into()',
        f"reasoning: {reasoning}",
        f"input: {parse_input_array(input_arr)}",
        f"cost: {parse_cost(cost_str)}",
        f"context_window: {context_window}",
        f"max_tokens: {max_tokens}",
    ]

    if thinking_str:
        tl = parse_thinking_map(thinking_str)
        parts.append(f"thinking_level_map: {tl if tl.startswith('Some') else 'None'}")
    else:
        parts.append("thinking_level_map: None")

    parts.append(f"headers: {parse_headers(headers_str) if headers_str else 'None'}")

    inner = ",\n        ".join(parts)
    return f"        Model {{\n        {inner}\n    }}"


def mangle(s: str) -> str:
    """Turn a string into a valid Rust identifier."""
    ident = re.sub(r"[^a-zA-Z0-9_]", "_", s)
    if ident and ident[0].isdigit():
        ident = "_" + ident
    if not ident:
        ident = "_"
    keywords = {
        "as", "break", "const", "continue", "crate", "else", "enum", "extern",
        "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
        "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
        "super", "trait", "true", "type", "unsafe", "use", "where", "while",
        "async", "await", "dyn",
    }
    return ident + "_" if ident in keywords else ident


# ---------------------------------------------------------------------------
# Rust source generation (per-provider files + aggregator)
# ---------------------------------------------------------------------------

def generate_provider_file(
    provider_name: str,
    models: list[dict[str, str]],
) -> str:
    """Generate one per-provider Rust source file."""
    has_thinking = any(m.get("thinkingLevelMap") for m in models)

    lines = []
    lines.append(f"//! Auto-generated model catalogue for `{provider_name}`.")
    lines.append("//! Do not edit manually — re-run `scripts/generate_models.py`.")
    lines.append("")
    lines.append("use std::collections::HashMap;")
    lines.append("")
    if has_thinking:
        lines.append("use crate::types::{Api, Model, ModelCost, InputModality, ModelThinkingLevel};")
    else:
        lines.append("use crate::types::{Api, Model, ModelCost, InputModality};")
    lines.append("")
    lines.append(f"/// Model catalogue for the `{provider_name}` provider.")
    lines.append("pub fn models() -> HashMap<String, Model> {")
    lines.append("    let mut map: HashMap<String, Model> = HashMap::new();")
    lines.append("")

    for fields in models:
        raw_id = fields.get("id", "unknown")
        mid = raw_id.strip('"') if raw_id.startswith('"') else raw_id
        rust_model = model_to_rust(fields, mid)
        lines.append("    map.insert(")
        lines.append(f'        "{esc(mid)}".into(),')
        lines.append(rust_model + ",")
        lines.append("    );")
        lines.append("")

    lines.append("    map")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def generate_mod_rs(providers: dict[str, list[dict[str, str]]]) -> str:
    """Generate providers/model_data/mod.rs."""
    lines = []
    lines.append("//! Auto-generated per-provider model data modules.")
    lines.append("//! Do not edit manually — re-run `scripts/generate_models.py`.")
    lines.append("")
    for name in sorted(providers.keys()):
        ident = mangle(name) + "_models"
        lines.append(f"pub mod {ident};")
    lines.append("")
    return "\n".join(lines)


def generate_aggregator(providers: dict[str, list[dict[str, str]]]) -> str:
    """Generate models_generated.rs — calls each per-provider function."""
    lines = []
    lines.append("//! Mirror of `packages/ai/src/models.generated.ts`.")
    lines.append("//!")
    lines.append("//! Auto-generated by `rust/scripts/generate_models.py`.")
    lines.append("//! Do not edit manually — re-run the generator when model data changes.")
    lines.append("")
    lines.append("use std::collections::HashMap;")
    lines.append("")
    lines.append("use crate::types::Model;")
    lines.append("")

    imports = []
    for name in sorted(providers.keys()):
        ident = mangle(name) + "_models"
        imports.append(ident)
    lines.append("use crate::providers::model_data::{")
    lines.append("    " + ",\n    ".join(imports) + ",")
    lines.append("};")

    lines.append("")

    lines.append("/// Build the provider → (model id → Model) registry.")
    lines.append("pub fn models() -> HashMap<String, HashMap<String, Model>> {")
    lines.append("    let mut map: HashMap<String, HashMap<String, Model>> = HashMap::new();")
    lines.append("")

    for name in sorted(providers.keys()):
        ident = mangle(name) + "_models"
        model_count = len(providers[name])
        lines.append(f"    // {name} ({model_count} models)")
        lines.append(f'    map.insert("{name}".into(), {ident}::models());')
        lines.append("")

    lines.append("    map")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    # 1. Try canonical monolithic source first, then fall back to per-provider files
    source_mode: str  # "monolith" or "perfile"
    all_providers: dict[str, list[dict[str, str]]] = {}

    if TS_MONOLITH_CANONICAL.exists():
        source_mode = "monolith"
        print(f"Using canonical monolithic source: {TS_MONOLITH_CANONICAL}", file=sys.stderr)
        all_providers = parse_monolithic_ts(TS_MONOLITH_CANONICAL)
    elif TS_AGGREGATOR_FALLBACK.exists():
        source_mode = "perfile"
        print(f"Falling back to per-provider files in: {TS_DIR_FALLBACK}", file=sys.stderr)
        provider_exports = parse_aggregator(TS_AGGREGATOR_FALLBACK)
        if not provider_exports:
            print(f"ERROR: could not parse {TS_AGGREGATOR_FALLBACK}", file=sys.stderr)
            sys.exit(1)
        for provider_name in sorted(provider_exports.keys()):
            ts_file = TS_DIR_FALLBACK / f"{provider_name}.models.ts"
            if not ts_file.exists():
                print(f"  WARNING: missing {ts_file.name}, skipping {provider_name}", file=sys.stderr)
                continue
            models = parse_perfile_ts(ts_file)
            all_providers[provider_name] = models
    else:
        print(f"ERROR: no TS source found", file=sys.stderr)
        sys.exit(1)

    if not all_providers:
        print("ERROR: no providers found in TS source", file=sys.stderr)
        sys.exit(1)

    total = sum(len(ms) for ms in all_providers.values())
    print(f"Found {len(all_providers)} providers, {total} models", file=sys.stderr)
    for name, models in sorted(all_providers.items()):
        print(f"  {name}: {len(models)} models", file=sys.stderr)

    # 2. Write per-provider Rust files
    RS_OUT_DIR.mkdir(parents=True, exist_ok=True)
    generated_files: list[Path] = []

    for name, models in sorted(all_providers.items()):
        ident = mangle(name) + "_models"
        path = RS_OUT_DIR / f"{ident}.rs"
        source = generate_provider_file(name, models)
        path.write_text(source, encoding="utf-8")
        generated_files.append(path)
        print(f"  Wrote {path.name} ({len(models)} models, {len(source.splitlines())} lines)", file=sys.stderr)

    # 3. Write mod.rs
    mod_path = RS_OUT_DIR / "mod.rs"
    mod_source = generate_mod_rs(all_providers)
    mod_path.write_text(mod_source, encoding="utf-8")
    generated_files.append(mod_path)
    print(f"  Wrote {mod_path.name}", file=sys.stderr)

    # 4. Write aggregator (models_generated.rs)
    agg_source = generate_aggregator(all_providers)
    RS_AGGREGATOR.write_text(agg_source, encoding="utf-8")
    print(f"\nWrote aggregator: {RS_AGGREGATOR.name} ({len(agg_source.splitlines())} lines)", file=sys.stderr)

    print(f"\nDone. Generated {len(generated_files)} files.", file=sys.stderr)


if __name__ == "__main__":
    main()
