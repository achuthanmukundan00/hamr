---
name: collaborative-design
description: Use when an ambiguous, visual, product, UX, TUI, architecture, or planning task would benefit from collaborative exploration before implementation
---

# Collaborative Design

Use this before locking a design for new behavior, UI/TUI, architecture, plans, comparisons, diagrams, reports, or anything easier to critique as an artifact than prose.

## Workflow

1. **Clarify the decision.** State the open questions, constraints, and what feedback you need from the user.
2. **Create an HTML artifact** in `.askr/artifacts/<topic>.html`. It may be a plan, table, decision matrix, UI/TUI mock, flow, architecture diagram, or annotated report.
3. **Match existing aesthetics first.** Inspect the target product/project for design tokens, CSS variables, Tailwind/DaisyUI config, component styles, brand assets, terminal palette, spacing, and typography. Only use a generic fallback when no local style exists.
4. **Use Lavish for review when available.** Prefer a local install; use `npx` only when network use is acceptable:
   ```bash
   artifact=.askr/artifacts/<topic>.html
   if command -v lavish-axi >/dev/null; then
     lavish-axi "$artifact" && lavish-axi poll "$artifact"
   elif command -v askr-lavish >/dev/null; then
     askr-lavish "$artifact" && askr-lavish poll "$artifact"
   elif command -v npx >/dev/null && [ "${HAMR_OFFLINE:-}" != "1" ]; then
     npx -y lavish-axi "$artifact" && npx -y lavish-axi poll "$artifact"
   else
     echo "Open or share: $artifact"
   fi
   ```
5. **Respond to feedback.** Apply annotations, rerun `poll --agent-reply "<summary>"`, and loop until the design is accepted or the user asks to stop.
6. **Move to execution.** For build work, hand the accepted design to `planning-and-execution`; for fixes, use `systematic-debugging` as needed.

## Lavish Rules

- For flows/state/architecture/sequence diagrams, open Lavish's `diagram` playbook and use Mermaid unless custom SVG is truly needed.
- Fix fresh error-severity `layout_warnings` before asking the user to review; disclose persistent low-severity warnings instead of looping forever.
- Do not reopen a session the user ended unless they ask or something important needs visual attention.
- Keep the artifact useful standalone: relative local assets, semantic sections, clear headings, and no hidden critical state.

## Tool Availability

- `lavish-axi` on PATH is preferred because it is local and deterministic.
- `askr-lavish` is optional. It exists only when AskR is installed as a full package; Hamr's bundled-skills path does not include the bin shim.
- `npx -y lavish-axi` requires npm/npx, network access, and live registry code. Do not use it in CI/offline/reproducible runs unless the user explicitly accepts that tradeoff.
- If no Lavish command is available, do not block the workflow. Present the artifact path directly to the user for manual review.
