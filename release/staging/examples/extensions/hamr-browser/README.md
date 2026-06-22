# Hamr Browser Extension

Hamr Browser gives the Hamr coding agent an isolated, visible browser window it owns for demos.

On launch it prints:

> Opening isolated Hamr Browser. Your normal browser profile is untouched.

## Standard Hamr Install

Hamr Browser is bundled with the standard Hamr package. The extension and skill load by default.

The first time a browser tool needs Playwright, Hamr asks before installing browser dependencies into the Hamr user data directory. The normal Hamr install stays lightweight until the browser is used.

For local development, you can still try this package explicitly:

```bash
hamr -e ./packages/coding-agent/examples/extensions/hamr-browser
```

Inside Hamr, launch the browser with:

```text
/browser
```

or ask the agent to use the browser tools.

## Tools

- `browser_launch({ url? })`
- `browser_open_url({ url })`
- `browser_snapshot({})`
- `browser_click({ target })`
- `browser_type({ target, text })`
- `browser_press({ key })`
- `browser_scroll({ direction_or_pixels })`
- `browser_wait({ ms })`
- `browser_screenshot({})`
- `browser_close({})`

## Target Syntax

Targets accept:

- `css=#submit`
- `text=Continue`
- `role=button:Submit`
- plain visible text such as `Continue`

CSS-like strings such as `#submit`, `.primary`, or `[name="q"]` are treated as CSS selectors.

## Profile and Artifacts

Default profile directory:

- macOS/Linux: `~/.hamr/browser-profile`
- Windows: `%LOCALAPPDATA%\\Hamr\\browser-profile`, with a home-directory fallback

Screenshots are saved under:

```text
~/.hamr/browser-artifacts/screenshots
```

## Configuration

Environment variables:

- `HAMR_BROWSER_PROFILE_DIR`: override the persistent profile directory.
- `HAMR_BROWSER_SCREENSHOT_DIR`: override screenshot output directory.
- `HAMR_BROWSER_CHANNEL=chrome`: use installed Chrome instead of bundled Chromium.
- `HAMR_BROWSER_USE_CHROME=1`: shorthand for Chrome channel.
- `HAMR_BROWSER_HEADLESS=1`: run headless, mainly for CI.
- `HAMR_BROWSER_DEPS_DIR`: override where first-use Playwright dependencies are installed.

## CLI Note

This package exposes a `/browser` extension command and agent tools. A literal `hamr browser` subcommand would require modifying Hamr CLI internals.
