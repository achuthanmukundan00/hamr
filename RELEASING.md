# Releasing `@skaft/hamr`

Hamr is developed as a 4-package workspace (`@hamr/tui`, `@hamr/ai`, `@hamr/agent`,
`@hamr/coding-agent`) but **ships as a single npm package: `@skaft/hamr`**.

We do not publish the workspace packages individually. Instead, the release
script vendors the three `@hamr/*` libraries into `@skaft/hamr`'s `node_modules`
and marks them as `bundledDependencies`. A user's `npm install -g @skaft/hamr`
is therefore byte-identical to the dev runtime — no JS bundling, so the image
worker, wasm, theme assets, and `jiti` extension loading all behave exactly as
they do in development.

```
packages/coding-agent  ──(rename @skaft/hamr + bundle @hamr/* libs)──▶  releases/skaft-hamr-<ver>.tgz
```

## TL;DR

```bash
npm run release          # build all packages, stage @skaft/hamr, pack, then verify the tarball
# then, when ready to ship (see Publish):
npm publish releases/skaft-hamr-<ver>.tgz --access public
```

## Scripts

| Command | What it does |
| --- | --- |
| `npm run release:build` | `scripts/build-release.mjs` — builds all 4 packages, stages `@skaft/hamr` (rewritten manifest + vendored `@hamr/*` libs), runs `npm pack`, drops the tarball in `releases/`. |
| `npm run release:verify` | `scripts/verify-pack.sh` — installs the newest tarball into a throwaway project with **npm** (a user's path) and smoke-tests the bundled libs + `hamr --version` / `--help`. |
| `npm run release` | build + verify in one shot. |

`release:build` accepts `--no-build` to reuse the existing `dist/` (faster iteration).

## Pre-release checklist

1. **Land all bug fixes** and run `npm run verify` (typecheck + lint + build + test) green.
2. **Bump the version** in `packages/coding-agent/package.json` (this is the single
   source of version truth; the release script reads it). Keep `CHANGELOG.md` in
   `packages/coding-agent/` updated.
3. **Scrub remaining pi-fork seams** (branding only — no behavior change):
   - `src/core/tools/bash.ts` → `tempFilePrefix: "pi-bash"` → `"hamr-bash"`
   - `src/core/bash-executor.ts` → `` `pi-bash-${id}.log` `` → `` `hamr-bash-${id}.log` ``
     (these surface in the bash truncation footer the user sees)
   - `src/cli/args.ts` → `--help` text mentions `PI_SHARE_VIEWER_URL` and
     `https://pi.dev/session/`; rebrand the help text (keep the env var as a
     silent fallback if you don't want to change behavior).
   - **Keep** the `pi-mono` migration shims (`src/migrations.ts`,
     `src/utils/changelog.ts`) — they are functional (they migrate old configs),
     not branding.
   - Docs (`packages/coding-agent/docs`, README `pi-mono`/`pi-share-hf` links) are
     intentionally **out of scope for the alpha**.
4. **Build + verify the tarball**: `npm run release` → expect `ALL CHECKS PASSED`.

## Publish (do together)

The package is scoped to the **`@skaft`** npm org — you must own it and be logged in.

```bash
# 1. Authenticate (once per machine)
npm login                       # ensure the account has publish rights to @skaft
npm whoami                      # sanity check

# 2. Publish the verified tarball (scoped packages need --access public)
npm publish releases/skaft-hamr-<ver>.tgz --access public

# 3. Tag the release in git
git tag v<ver>
git push origin v<ver>
```

After publishing, confirm a clean machine can install it:

```bash
npm install -g @skaft/hamr && hamr --version
```

## Notes / footguns

- **Never run `npm publish` from inside `packages/*`** — those are the workspace
  packages with `"*"` deps and would publish the wrong thing. Only ever publish
  the staged tarball from `releases/`.
- The bundled `@hamr/*` libs are pinned to their built versions at pack time; the
  union of their real (non-`@hamr`) dependencies is declared on `@skaft/hamr` so
  npm hoists them and the vendored libs resolve.
- Cross-platform: this is a plain Node package, so the same tarball runs on Linux
  (x64/arm64) and macOS — no per-arch builds needed.
