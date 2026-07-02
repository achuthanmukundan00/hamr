#!/usr/bin/env bash
# Bundle askr skills and bin into dist/askr/ at build time.
# Fetches the latest askr release tag and copies skills + CLI launchers.
set -euo pipefail

ASKR_REPO="git@github.com:skaft-software/askr.git"
DIST_BASE="dist/askr"

# Resolve the latest release tag, fall back to main
echo "[bundle-askr] fetching latest askr release..."
LATEST_TAG=$(git ls-remote --tags --sort=-version:refname "$ASKR_REPO" 'v*' 2>/dev/null | head -1 | awk '{print $2}' | sed 's|refs/tags/||')

if [ -z "$LATEST_TAG" ]; then
  echo "[bundle-askr] no release tags found, using main branch"
  REF="main"
else
  echo "[bundle-askr] latest release: $LATEST_TAG"
  REF="$LATEST_TAG"
fi

TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

git clone --depth 1 --branch "$REF" "$ASKR_REPO" "$TEMP_DIR"

# Bundle skills
rm -rf "$DIST_BASE/skills"
mkdir -p "$DIST_BASE/skills"
cp -r "$TEMP_DIR/skills/"* "$DIST_BASE/skills/"

# Bundle CLI launcher scripts (askr-lavish, no-mistakes, etc.)
if [ -d "$TEMP_DIR/bin" ]; then
  rm -rf "$DIST_BASE/bin"
  mkdir -p "$DIST_BASE/bin"
  cp -r "$TEMP_DIR/bin/"* "$DIST_BASE/bin/"
fi

# Bundle extension (as session-start/bootstrap shim)
if [ -d "$TEMP_DIR/.hamr" ]; then
  rm -rf "$DIST_BASE/.hamr"
  mkdir -p "$DIST_BASE/.hamr"
  cp -r "$TEMP_DIR/.hamr/"* "$DIST_BASE/.hamr/"
fi

echo "[bundle-askr] done — $(ls -1 "$DIST_BASE/skills" | wc -l) skills, $(ls -1 "$DIST_BASE/bin" 2>/dev/null | wc -l) launchers, $(ls -1 "$DIST_BASE/.hamr/extensions" 2>/dev/null | wc -l) extensions bundled"
