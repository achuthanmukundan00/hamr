# Skill Authoring Best Practices

Condensed from Anthropic guidance. Use official docs for latest full details.

## Principles

- Concise. Context is shared with system prompt, history, metadata, request, and loaded skills.
- Assume agent is smart. Add only missing, task-specific guidance.
- Match freedom to risk:
  - high freedom: many valid approaches, context decides
  - medium: preferred pattern with parameters
  - low: fragile operation; exact commands/scripts
- Test on every target model. Smaller models may need more explicit guardrails; larger models need less explanation.

## Structure

Frontmatter:
- `name`: max 64 chars
- `description`: max 1024 chars, third person, discovery trigger

Name with consistent gerund/action form. Avoid vague names.

Description: specific triggers and keywords. Good: file types, symptoms, user phrases, tools. Bad: "helps with data".

SKILL.md is table of contents plus core instructions. Split heavy refs, scripts, templates, assets into separate files loaded only as needed.

## Progressive Disclosure

Use:

```text
skill/
  SKILL.md      # trigger, core workflow
  reference.md  # big docs, only as needed
  scripts/      # executable tools
  templates/    # reusable artifacts
```

Keep SKILL.md small; link support files by task condition.

## Content Rules

- One excellent example beats many.
- Prefer commands/scripts over prose for exact operations.
- State inputs/outputs and failure modes.
- Put install/setup prerequisites near use.
- Include "when not to use" when confusion likely.
- Avoid generic explanations agents already know.

## Testing

Test real tasks, not quiz recall. Include happy path, edge path, and pressure path. Verify the agent can find the skill from description and use the body correctly.
