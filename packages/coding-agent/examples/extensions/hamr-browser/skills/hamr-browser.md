---
name: hamr-browser
description: Use when the user wants Hamr to visibly control an isolated browser for demos, web navigation, screenshots, or page interaction.
---

# Hamr Browser

Use Hamr Browser tools to control the isolated visible browser profile owned by Hamr.

If browser dependencies are missing, `browser_launch` or another browser tool will ask the user before installing Playwright into Hamr's user data directory.

## Rules

- Start with `browser_launch` or `browser_open_url`.
- Use `browser_snapshot` before deciding what to click or type.
- Prefer robust targets:
  - `css=#id` or `css=[name="q"]` when known.
  - `role=button:Submit` for accessible controls.
  - `text=Continue` or plain visible text for demos.
- Use `browser_screenshot` when the user asks for a visual artifact.
- Use `browser_close` when done.

## Safety

Hamr Browser uses an isolated Hamr-owned profile. Do not claim it controls or reads the user's normal browser profile.
